#![doc = "This file was automatically generated by the varlink rust generator"]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
use serde_derive::{Deserialize, Serialize};
use serde_json;
use std::io::BufRead;
use std::sync::{Arc, RwLock};
use varlink::{self, CallTrait};
#[allow(dead_code)]
#[derive(Clone, PartialEq, Debug)]
pub enum ErrorKind {
    Varlink_Error,
    VarlinkReply_Error,
}
impl ::std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            ErrorKind::Varlink_Error => write!(f, "Varlink Error"),
            ErrorKind::VarlinkReply_Error => write!(f, "Varlink error reply"),
        }
    }
}
pub struct Error(
    pub ErrorKind,
    pub Option<Box<dyn std::error::Error + 'static + Send + Sync>>,
    pub Option<&'static str>,
);
impl Error {
    #[allow(dead_code)]
    pub fn kind(&self) -> &ErrorKind {
        &self.0
    }
}
impl From<ErrorKind> for Error {
    fn from(e: ErrorKind) -> Self {
        Error(e, None, None)
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.1
            .as_ref()
            .map(|e| e.as_ref() as &(dyn std::error::Error + 'static))
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}
impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use std::error::Error as StdError;
        if let Some(ref o) = self.2 {
            std::fmt::Display::fmt(o, f)?;
        }
        std::fmt::Debug::fmt(&self.0, f)?;
        if let Some(e) = self.source() {
            std::fmt::Display::fmt("\nCaused by:\n", f)?;
            std::fmt::Debug::fmt(&e, f)?;
        }
        Ok(())
    }
}
#[allow(dead_code)]
pub type Result<T> = std::result::Result<T, Error>;
impl From<varlink::Error> for Error {
    fn from(e: varlink::Error) -> Self {
        match e.kind() {
            varlink::ErrorKind::VarlinkErrorReply(r) => Error(
                ErrorKind::from(r),
                Some(Box::from(e)),
                Some(concat!(file!(), ":", line!(), ": ")),
            ),
            _ => Error(
                ErrorKind::Varlink_Error,
                Some(Box::from(e)),
                Some(concat!(file!(), ":", line!(), ": ")),
            ),
        }
    }
}
#[allow(dead_code)]
impl Error {
    pub fn source_varlink_kind(&self) -> Option<&varlink::ErrorKind> {
        use std::error::Error as StdError;
        let mut s: &dyn StdError = self;
        while let Some(c) = s.source() {
            let k = self
                .source()
                .and_then(|e| e.downcast_ref::<varlink::Error>())
                .and_then(|e| Some(e.kind()));
            if k.is_some() {
                return k;
            }
            s = c;
        }
        None
    }
}
impl From<&varlink::Reply> for ErrorKind {
    #[allow(unused_variables)]
    fn from(e: &varlink::Reply) -> Self {
        match e {
            _ => ErrorKind::VarlinkReply_Error,
        }
    }
}
pub trait VarlinkCallError: varlink::CallTrait {}
impl<'a> VarlinkCallError for varlink::Call<'a> {}
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum r#Failure_kind {
    r#io,
    r#spawn,
    r#exit,
    r#output,
}
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct r#Failure {
    pub r#kind: Failure_kind,
    pub r#msg: Option<String>,
    pub r#cmd: Option<String>,
    pub r#status: Option<i64>,
    pub r#logs: Option<Vec<String>>,
}
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct r#Outcome {
    pub r#project_root: String,
}
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum r#Reason_kind {
    r#project_added,
    r#ping_received,
    r#files_changed,
    r#unknown,
}
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct r#Reason {
    pub r#kind: Reason_kind,
    pub r#project: Option<ShellNix>,
    pub r#files: Option<Vec<String>>,
    pub r#debug: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct r#ShellNix {
    pub r#path: String,
}
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct WatchShell_Reply {}
impl varlink::VarlinkReply for WatchShell_Reply {}
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct WatchShell_Args {
    pub r#shell_nix: ShellNix,
}
pub trait Call_WatchShell: VarlinkCallError {
    fn reply(&mut self) -> varlink::Result<()> {
        self.reply_struct(varlink::Reply::parameters(None))
    }
}
impl<'a> Call_WatchShell for varlink::Call<'a> {}
pub trait VarlinkInterface {
    fn watch_shell(
        &self,
        call: &mut dyn Call_WatchShell,
        r#shell_nix: ShellNix,
    ) -> varlink::Result<()>;
    fn call_upgraded(
        &self,
        _call: &mut varlink::Call,
        _bufreader: &mut dyn BufRead,
    ) -> varlink::Result<Vec<u8>> {
        Ok(Vec::new())
    }
}
pub trait VarlinkClientInterface {
    fn watch_shell(
        &mut self,
        r#shell_nix: ShellNix,
    ) -> varlink::MethodCall<WatchShell_Args, WatchShell_Reply, Error>;
}
#[allow(dead_code)]
pub struct VarlinkClient {
    connection: Arc<RwLock<varlink::Connection>>,
}
impl VarlinkClient {
    #[allow(dead_code)]
    pub fn new(connection: Arc<RwLock<varlink::Connection>>) -> Self {
        VarlinkClient { connection }
    }
}
impl VarlinkClientInterface for VarlinkClient {
    fn watch_shell(
        &mut self,
        r#shell_nix: ShellNix,
    ) -> varlink::MethodCall<WatchShell_Args, WatchShell_Reply, Error> {
        varlink::MethodCall::<WatchShell_Args, WatchShell_Reply, Error>::new(
            self.connection.clone(),
            "com.target.lorri.internal.WatchShell",
            WatchShell_Args { r#shell_nix },
        )
    }
}
#[allow(dead_code)]
pub struct VarlinkInterfaceProxy {
    inner: Box<dyn VarlinkInterface + Send + Sync>,
}
#[allow(dead_code)]
pub fn new(inner: Box<dyn VarlinkInterface + Send + Sync>) -> VarlinkInterfaceProxy {
    VarlinkInterfaceProxy { inner }
}
impl varlink::Interface for VarlinkInterfaceProxy {
    fn get_description(&self) -> &'static str {
        "# The interface `lorri daemon` exposes.\ninterface com.target.lorri.internal\n\n# WatchShell instructs the daemon to evaluate a Nix expression and re-evaluate\n# it when it or its dependencies change.\nmethod WatchShell(shell_nix: ShellNix) -> ()\n\n# ShellNix describes the Nix expression which evaluates to a development\n# environment.\ntype ShellNix (\n  # The absolute path of a Nix file specifying the project environment.\n  path: string\n)\n\ntype Reason (\n    kind: (project_added, ping_received, files_changed, unknown),\n    project: ?ShellNix, # only present if kind == project_added\n    files: ?[]string,   # only present if kind == files_changed\n    debug: ?string      # only present if kind == unknown\n)\n\ntype Outcome (\n    project_root: string\n)\n\ntype Failure (\n    kind: (io, spawn, exit, output),\n    msg: ?string,   # only present if kind in (io, spawn)\n    cmd: ?string,   # only present if kind in (spawn, exit)\n    status: ?int,   # only present if kind == exit\n    logs: ?[]string # only present if kind == exit\n)\n"
    }
    fn get_name(&self) -> &'static str {
        "com.target.lorri.internal"
    }
    fn call_upgraded(
        &self,
        call: &mut varlink::Call,
        bufreader: &mut dyn BufRead,
    ) -> varlink::Result<Vec<u8>> {
        self.inner.call_upgraded(call, bufreader)
    }
    fn call(&self, call: &mut varlink::Call) -> varlink::Result<()> {
        let req = call.request.unwrap();
        match req.method.as_ref() {
            "com.target.lorri.internal.WatchShell" => {
                if let Some(args) = req.parameters.clone() {
                    let args: WatchShell_Args = match serde_json::from_value(args) {
                        Ok(v) => v,
                        Err(e) => {
                            let es = format!("{}", e);
                            let _ = call.reply_invalid_parameter(es.clone());
                            return Err(
                                varlink::context!(varlink::ErrorKind::SerdeJsonDe(es)).into()
                            );
                        }
                    };
                    self.inner
                        .watch_shell(call as &mut dyn Call_WatchShell, args.r#shell_nix)
                } else {
                    call.reply_invalid_parameter("parameters".into())
                }
            }
            m => call.reply_method_not_found(String::from(m)),
        }
    }
}
