{ pkgs, LORRI_ROOT, rust }:
let
  inherit (import ./execline.nix { inherit pkgs; })
    writeExecline;

  # Write commands to script which aborts immediately if a command is not successful.
  # The status of the unsuccessful command is returned.
  allCommandsSucceed = name: commands: pipe commands [
    (pkgs.lib.concatMap (cmd: [ "if" [ cmd ] ]))
    (cmds: cmds ++ [ "true" ])
    (writeExecline name {})
  ];

  # shellcheck file
  shellcheck = file: writeExecline "lint-shellcheck" {} [
    "cd" LORRI_ROOT
    # TODO: echo is coming from context, clean out PATH before running checks
    "foreground" [ "echo" "shellchecking ${file}" ]
    "${pkgs.shellcheck}/bin/shellcheck" "--shell" "bash" file
  ];

  # the CI tests we want to run
  # Tests should not depend on each other (or block if they do),
  # so that they can run in parallel.
  # If a test changes files in the repository, sandbox it.
  tests = {

    shellcheck =
      let files = [
        "nix/bogus-nixpkgs/builder.sh"
        "src/ops/direnv/envrc.bash"
      ];
      in {
        description = "shellcheck ${pkgs.lib.concatStringsSep " and " files}";
        test = allCommandsSucceed "lint-shellcheck-all" (map shellcheck files);
      };

    cargo-fmt = {
      description = "cargo fmt was done";
      test = writeExecline "lint-cargo-fmt" {} [ "${rust}/bin/cargo" "fmt" "--" "--check" ];
    };

    cargo-test = {
      description = "run cargo test";
      test = writeExecline "cargo-test" {} [ "${rust}/bin/cargo" "test" ];
    };

    cargo-clippy = {
      description = "run cargo clippy";
      test = writeExecline "cargo-clippy" {} [
        "export" "RUSTFLAGS" "-D warnings"
        "${rust}/bin/cargo" "clippy"
      ];
    };

  };

  # Write a attrset which looks like
  # { "test description" = test-script-derviation }
  # to a script which can be read by `bats` (a simple testing framework).
  batsScript = name: tests: pipe tests [
    (pkgs.lib.mapAttrsToList
      # a bats test looks like:
      # @test "name of test" {
      #   … test code …
      # }
      # bats is very picky about the {} block (and the newlines).
      (_: test: "@test ${pkgs.lib.escapeShellArg test.description} {\n${test.test}\n}"))
    (pkgs.lib.concatStringsSep "\n")
    (pkgs.writeText "testsuite")
    (test-suite: writeExecline name {} [
      "${pkgs.bats}/bin/bats"
      test-suite ])
  ];

  testsuite = batsScript "run-testsuite" tests;

in {
  inherit
    testsuite tests;
}
