//! Builds a nix derivation file (like a `shell.nix` file).
//!
//! It is a wrapper around `nix-build`.
//!
//! Note: this does not build the Nix expression as-is.
//! It instruments various nix builtins in a way that we
//! can parse additional information from the `nix-build`
//! `stderr`, like which source files are used by the evaluator.

use cas::ContentAddressable;
use nix::StorePath;
use osstrlines;
use regex::Regex;
use std::any::Any;
use std::ffi::{OsStr, OsString};
use std::io::BufReader;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use {DrvFile, NixFile};

fn instrumented_instantiation(
    root_nix_file: &NixFile,
    cas: &ContentAddressable,
) -> Result<Info<DrvFile, GcRootTempDir>, Error> {
    // We're looking for log lines matching:
    //
    //     copied source '...' -> '/nix/store/...'
    //     evaluating file '...'
    //
    // to determine which files we should setup watches on.
    // Increasing verbosity by two levels via `-vv` satisfies that.

    let mut cmd = Command::new("nix-instantiate");

    let logged_evaluation_nix = cas.file_from_string(include_str!("./logged-evaluation.nix"))?;

    // TODO: see ::nix::CallOpts::paths for the problem with this
    let gc_root_dir = tempfile::TempDir::new()?;

    cmd.args(&[
        // verbose mode prints the files we track
        OsStr::new("-vv"),
        // we add a temporary indirect GC root
        OsStr::new("--add-root"),
        gc_root_dir.path().join("result").as_os_str(),
        OsStr::new("--indirect"),
        OsStr::new("--argstr"),
        // runtime nix paths to needed dependencies that come with lorri
        OsStr::new("runTimeClosure"),
        OsStr::new(crate::RUN_TIME_CLOSURE),
        // the source file
        OsStr::new("--argstr"),
        OsStr::new("src"),
        root_nix_file.as_os_str(),
        // instrumented by `./logged-evaluation.nix`
        OsStr::new("--"),
        &logged_evaluation_nix.as_os_str(),
    ])
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());

    debug!("$ {:?}", cmd);

    let mut child = cmd.spawn()?;

    let stdout = child
        .stdout
        .take()
        .expect("we must be able to access the stdout of nix-build");
    let stderr = child
        .stderr
        .take()
        .expect("we must be able to access the stderr of nix-build");

    let stderr_results: thread::JoinHandle<std::io::Result<Vec<LogDatum>>> =
        thread::spawn(move || {
            osstrlines::Lines::from(BufReader::new(stderr))
                .map(|line| line.map(parse_evaluation_line))
                .collect::<Result<Vec<LogDatum>, _>>()
        });

    let build_products: thread::JoinHandle<std::io::Result<Vec<DrvFile>>> =
        thread::spawn(move || {
            osstrlines::Lines::from(BufReader::new(stdout))
                .map(|line| line.map(|os_string| DrvFile::from(PathBuf::from(os_string))))
                .collect::<Result<Vec<DrvFile>, _>>()
        });

    let (exec_result, mut build_products, results) = (
        child.wait()?,
        build_products.join()??,
        stderr_results.join()??,
    );

    // TODO: this can move entirely into the stderr thread,
    // meaning we don’t have to keep the outputs in memory (fold directly)

    // iterate over all lines, parsing out the ones we are interested in
    let (paths, log_lines): (Vec<PathBuf>, Vec<OsString>) =
        results
            .into_iter()
            .fold((vec![], vec![]), |(mut paths, mut log_lines), result| {
                match result {
                    LogDatum::Source(src) => {
                        paths.push(src);
                    }
                    LogDatum::Text(line) => log_lines.push(line),
                    LogDatum::NonUtf(line) => log_lines.push(line),
                };

                (paths, log_lines)
            });

    if !exec_result.success() {
        return Ok(Info::Failure(Failure {
            exec_result,
            log_lines,
        }));
    }

    assert!(
        build_products.len() == 1,
        "got more or less than one build product from logged_evaluation.nix: {:#?}",
        build_products
    );
    let shell_gc_root = build_products.pop().unwrap();

    Ok(Info::Success(Success {
        gc_root_temp_dir: GcRootTempDir(gc_root_dir),
        output_paths: OutputPaths { shell_gc_root },
        paths,
        log_lines,
    }))
}

/// Opaque type to keep a temporary GC root directory alive.
/// Once it is dropped, the GC root is removed.
/// Copied from `nix`, because the type should stay opaque.
#[derive(Debug)]
struct GcRootTempDir(tempfile::TempDir);

/// Builds the Nix expression in `root_nix_file`.
///
/// Instruments the nix file to gain extra information,
/// which is valuable even if the build fails.
pub fn run(
    root_nix_file: &NixFile,
    cas: &ContentAddressable,
) -> Result<Info<StorePath, ::nix::GcRootTempDir>, Error> {
    let inst_info = instrumented_instantiation(root_nix_file, cas)?;
    match inst_info {
        Info::Success(mut s) => {
            let drvs = s.output_paths.clone();
            let mut loglines: Vec<OsString> = vec![];
            let realized = ::nix::CallOpts::file(drvs.shell_gc_root.as_path())
                .stderr(&mut |line: &OsStr| s.log_lines.push(line.to_owned()))
                .path()?;
            println!("{:#?}", loglines);
            Ok(Info::Success(Success {
                output_paths: OutputPaths {
                    shell_gc_root: realized.0,
                },
                gc_root_temp_dir: realized.1,
                paths: s.paths,
                // TODO: we are passing the instantiation stderr here,
                // but we really want to get the CallOpts stderr
                log_lines: s.log_lines,
            }))
        }
        Info::Failure(f) => Ok(Info::Failure(f)),
    }
}

/// Classifies the output of nix-instantiate -vv.
#[derive(Debug, PartialEq, Clone)]
enum LogDatum {
    /// Nix source file (which should be tracked)
    Source(PathBuf),
    /// Arbitrary text (which we couldn’t otherwise classify)
    Text(OsString),
    /// Text which we coudn’t decode from UTF-8
    NonUtf(OsString),
}

/// Examine a line of output and extract interesting log items in to
/// structured data.
fn parse_evaluation_line<T>(line: T) -> LogDatum
where
    T: AsRef<OsStr>,
{
    lazy_static! {
        static ref EVAL_FILE: Regex =
            Regex::new("^evaluating file '(?P<source>.*)'$").expect("invalid regex!");
        static ref COPIED_SOURCE: Regex =
            Regex::new("^copied source '(?P<source>.*)' -> '(?:.*)'$").expect("invalid regex!");
        static ref LORRI_READ: Regex =
            Regex::new("^trace: lorri read: '(?P<source>.*)'$").expect("invalid regex!");
    }

    match line.as_ref().to_str() {
        // If we can’t decode the output line to an UTF-8 string,
        // we cannot match against regexes, so just pass it through.
        None => LogDatum::NonUtf(line.as_ref().to_owned()),
        Some(linestr) => {
            // Lines about evaluating a file are much more common, so looking
            // for them first will reduce comparisons.
            if let Some(matches) = EVAL_FILE.captures(&linestr) {
                LogDatum::Source(PathBuf::from(&matches["source"]))
            } else if let Some(matches) = COPIED_SOURCE.captures(&linestr) {
                LogDatum::Source(PathBuf::from(&matches["source"]))
            } else if let Some(matches) = LORRI_READ.captures(&linestr) {
                LogDatum::Source(PathBuf::from(&matches["source"]))
            } else {
                LogDatum::Text(line.as_ref().to_owned())
            }
        }
    }
}

/// The results of an individual instantiation/build.
/// Even if the exit code is not 0, there is still
/// valuable information in the output, like new paths
/// to watch.
#[derive(Debug)]
pub enum Info<T, TempDir> {
    /// Nix ran successfully.
    Success(Success<T, TempDir>),
    /// Nix returned a failing status code.
    Failure(Failure),
}

/// A successful Nix run.
#[derive(Debug)]
pub struct Success<T, TempDir> {
    /// See `OutputPaths`
    // TODO: move back to `OutputPaths<T>`
    pub output_paths: OutputPaths<T>,

    /// Handle that keeps the temporary GC root directory around
    pub gc_root_temp_dir: TempDir,

    // TODO: rename to `sources` (it’s the input sources we have to watch)
    /// A list of paths examined during the evaluation
    pub paths: Vec<PathBuf>,

    // INFO: only used in test so far
    /// A list of stderr log lines
    pub log_lines: Vec<OsString>,
}

/// A failing Nix run.
#[derive(Debug)]
pub struct Failure {
    /// The error status code
    exec_result: std::process::ExitStatus,

    /// A list of stderr log lines
    pub log_lines: Vec<OsString>,
}

/// Output paths generated by `logged-evaluation.nix`
#[derive(Debug, Clone)]
pub struct OutputPaths<T> {
    /// Shell path modified to work as a gc root
    pub shell_gc_root: T,
}

/// Possible errors from an individual evaluation
#[derive(Debug)]
pub enum Error {
    /// Executing nix-instantiate failed
    Instantiate(std::io::Error),

    /// Executing nix-build failed
    Build(::nix::OnePathError),

    /// Failed to spawn a log processing thread
    ThreadFailure(std::boxed::Box<(dyn std::any::Any + std::marker::Send + 'static)>),
}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::Instantiate(e)
    }
}
impl From<::nix::OnePathError> for Error {
    fn from(e: ::nix::OnePathError) -> Error {
        Error::Build(e)
    }
}
impl From<Box<dyn Any + Send + 'static>> for Error {
    fn from(e: std::boxed::Box<(dyn std::any::Any + std::marker::Send + 'static)>) -> Error {
        Error::ThreadFailure(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cas::ContentAddressable;
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStrExt;
    use std::path::PathBuf;

    #[test]
    fn test_evaluation_line_to_path_evaluation() {
        assert_eq!(
            parse_evaluation_line("evaluating file '/nix/store/zqxha3ax0w771jf25qdblakka83660gr-source/lib/systems/for-meta.nix'"),
            LogDatum::Source(PathBuf::from("/nix/store/zqxha3ax0w771jf25qdblakka83660gr-source/lib/systems/for-meta.nix"))
        );

        assert_eq!(
            parse_evaluation_line("copied source '/nix/store/zqxha3ax0w771jf25qdblakka83660gr-source/pkgs/stdenv/generic/default-builder.sh' -> '/nix/store/9krlzvny65gdc8s7kpb6lkx8cd02c25b-default-builder.sh'"),
            LogDatum::Source(PathBuf::from("/nix/store/zqxha3ax0w771jf25qdblakka83660gr-source/pkgs/stdenv/generic/default-builder.sh"))
        );

        assert_eq!(
            parse_evaluation_line(
                "trace: lorri read: '/home/grahamc/projects/grahamc/lorri/nix/nixpkgs.json'"
            ),
            LogDatum::Source(PathBuf::from(
                "/home/grahamc/projects/grahamc/lorri/nix/nixpkgs.json"
            ))
        );

        assert_eq!(
            parse_evaluation_line(
                "downloading 'https://static.rust-lang.org/dist/channel-rust-stable.toml'..."
            ),
            LogDatum::Text(OsString::from(
                "downloading 'https://static.rust-lang.org/dist/channel-rust-stable.toml'..."
            ))
        );
    }

    #[test]
    fn non_utf8_nix_output() -> std::io::Result<()> {
        let tmp = tempfile::tempdir()?;
        let cas = ContentAddressable::new(tmp.path().to_owned())?;
        let drv = |name: &str, args: &str| {
            format!(
                r##"
derivation {{
  name = "{}";
  builder = "/bin/sh";
  allowSubstitutes = false;
  preferLocalBuild = true;
  system = builtins.currentSystem;
  # this is to make nix rebuild for every test
  random = builtins.currentTime;
  {}
}}"##,
                name, args
            )
        };

        let nix_drv = format!(
            r##"
let dep = {};
in {}
"##,
            drv(
                "dep",
                r##"
  args = [
    "-c"
    ''
    # non-utf8 sequence to stdout (which is nix stderr)
    printf '"\xab\xbc\xcd\xde\xde\xef"'
    echo > $out
    ''
  ];"##
            ),
            drv("shell", "inherit dep;")
        );

        print!("{}", nix_drv);

        let info = run(&::NixFile::from(cas.file_from_string(&nix_drv)?), &cas).unwrap();
        match info {
            Info::Success(i) => {
                let expect: OsString =
                    OsStr::from_bytes(b"\"\xAB\xBC\xCD\xDE\xDE\xEF\"").to_owned();
                assert!(i.log_lines.contains(&expect))
            }
            _ => panic!(),
        }
        Ok(())
    }

}
