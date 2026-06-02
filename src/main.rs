// `cargo-crushy` is a thin launcher: it sets `RUSTC_WORKSPACE_WRAPPER` and runs
// cargo. It deliberately links NO rustc internals (`rustc_driver`, `crushy_lints`)
// so it can launch under any consumer toolchain, then force crushy's matching
// toolchain for the actual check. Everything rustc-linked lives in `crushy-driver`.
// warn on lints, that are included in `rust-lang/rust`s bootstrap
#![warn(rust_2018_idioms, unused_lifetimes)]

mod toolchain;

use std::env;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{self, Command, exit};

fn show_help() {
    if writeln!(&mut anstream::stdout().lock(), "{}", help_message()).is_err() {
        exit(1);
    }
}

fn show_version() {
    let version_info = rustc_tools_util::get_version_info!();
    if writeln!(&mut anstream::stdout().lock(), "{version_info}").is_err() {
        exit(1);
    }
}

pub fn main() {
    // Check for version and help flags even when invoked as 'cargo-crushy'
    if env::args().any(|a| a == "--help" || a == "-h") {
        show_help();
        return;
    }

    if env::args().any(|a| a == "--version" || a == "-V") {
        show_version();
        return;
    }

    if let Some(pos) = env::args().position(|a| a == "--explain") {
        if let Some(lint) = env::args().nth(pos + 1) {
            process::exit(explain_via_driver(&lint));
        } else {
            show_help();
        }
        return;
    }

    if let Err(code) = process(env::args().skip(2)) {
        process::exit(code);
    }
}

struct CrushyCmd {
    cargo_subcommand: &'static str,
    args: Vec<String>,
    crushy_args: Vec<String>,
}

impl CrushyCmd {
    fn new<I>(mut old_args: I) -> Self
    where
        I: Iterator<Item = String>,
    {
        let mut cargo_subcommand = "check";
        let mut args = vec![];
        let mut crushy_args: Vec<String> = vec![];

        for arg in old_args.by_ref() {
            match arg.as_str() {
                "--fix" => {
                    cargo_subcommand = "fix";
                    continue;
                },
                "--no-deps" => {
                    crushy_args.push("--no-deps".into());
                    continue;
                },
                "--" => break,
                _ => {},
            }

            args.push(arg);
        }

        crushy_args.append(&mut (old_args.collect()));
        if cargo_subcommand == "fix" && !crushy_args.iter().any(|arg| arg == "--no-deps") {
            crushy_args.push("--no-deps".into());
        }

        Self {
            cargo_subcommand,
            args,
            crushy_args,
        }
    }

    fn into_std_cmd(self) -> Command {
        let crushy_args: String = self
            .crushy_args
            .iter()
            .fold(String::new(), |s, arg| s + arg + "__CRUSHY_HACKERY__");

        // Currently, `CRUSHY_TERMINAL_WIDTH` is used only to format "unknown field" error messages.
        let terminal_width = termize::dimensions().map_or(0, |(w, _)| w);

        // The driver is ABI-locked to the toolchain crushy was built with. When
        // the consumer is on a different toolchain, run the inner cargo under
        // crushy's via `rustup run`, so both cargo and the driver
        // (RUSTC_WORKSPACE_WRAPPER) resolve the matching librustc_driver. When
        // already compatible (or explicitly pinned), reuse the invoking cargo.
        let mut cmd = match toolchain::force() {
            Some(tk) => {
                // Run the inner cargo via `rustup run <tk>`, which sets PATH and
                // the dylib search path to crushy's toolchain so the driver
                // (RUSTC_WORKSPACE_WRAPPER) finds its librustc_driver. A bare
                // `cargo` can't be trusted: the invoking cargo may have put a
                // non-proxy toolchain's cargo first on PATH, which sets no dylib
                // path at all. Clear the inherited concrete RUSTC/CARGO (a
                // concrete RUSTC would keep the consumer's rustc and sysroot)
                // and export RUSTUP_TOOLCHAIN so any nested proxy agrees.
                let mut cmd = Command::new("rustup");
                cmd.arg("run")
                    .arg(&tk)
                    .arg("cargo")
                    .env("RUSTUP_TOOLCHAIN", &tk)
                    .env_remove("RUSTC")
                    .env_remove("CARGO")
                    // Isolate crushy's artifacts from the consumer's `target/`
                    // (used by `cargo build`/`cargo clippy` under *their*
                    // toolchain) so the two toolchains don't invalidate each
                    // other's caches on every alternation.
                    .env("CARGO_TARGET_DIR", forced_target_dir());
                cmd
            },
            None => Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".into())),
        };

        cmd.env("RUSTC_WORKSPACE_WRAPPER", driver_path())
            .env("CRUSHY_ARGS", crushy_args)
            .env("CRUSHY_TERMINAL_WIDTH", terminal_width.to_string())
            .arg(self.cargo_subcommand)
            .args(&self.args);

        cmd
    }
}

/// Where crushy puts build artifacts when it forces its own toolchain — kept
/// separate from the consumer's `target/` (used by `cargo build`/`cargo clippy`
/// under their toolchain) so the two never invalidate each other.
///
/// Anchored at the workspace's target dir (absolute), not a cwd-relative
/// `target/crushy`: in a workspace a relative path scatters into whichever
/// member dir crushy was invoked from, so `rm -rf target/crushy` from the root
/// wouldn't find it. Respects an explicit `CARGO_TARGET_DIR` if set.
fn forced_target_dir() -> PathBuf {
    if let Some(base) = env::var_os("CARGO_TARGET_DIR") {
        return PathBuf::from(base).join("crushy");
    }
    match workspace_root() {
        Some(root) => root.join("target").join("crushy"),
        None => PathBuf::from("target/crushy"),
    }
}

/// The workspace root (parent of the workspace `Cargo.toml`), via
/// `cargo locate-project`. `None` if it can't be determined.
fn workspace_root() -> Option<PathBuf> {
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let out = Command::new(cargo)
        .args(["locate-project", "--workspace", "--message-format", "plain"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let manifest = String::from_utf8(out.stdout).ok()?;
    Path::new(manifest.trim()).parent().map(Path::to_path_buf)
}

/// Path to the sibling `crushy-driver` binary (installed alongside this one).
fn driver_path() -> PathBuf {
    let mut path = env::current_exe()
        .expect("current executable path invalid")
        .with_file_name("crushy-driver");

    if cfg!(windows) {
        path.set_extension("exe");
    }

    path
}

/// Print a lint's docs by delegating to `crushy-driver --explain` under crushy's
/// toolchain. The driver links `crushy_lints`; this launcher must not (so it can
/// start under any toolchain), hence `rustup run` to load the matching driver.
fn explain_via_driver(lint: &str) -> i32 {
    Command::new("rustup")
        .args(["run", toolchain::pinned_nightly()])
        .arg(driver_path())
        .arg("--explain")
        .arg(lint)
        .status()
        .ok()
        .and_then(|s| s.code())
        .unwrap_or(1)
}

fn process<I>(old_args: I) -> Result<(), i32>
where
    I: Iterator<Item = String>,
{
    let cmd = CrushyCmd::new(old_args);

    let mut cmd = cmd.into_std_cmd();

    let exit_status = cmd
        .spawn()
        .expect("could not run cargo")
        .wait()
        .expect("failed to wait for cargo?");

    if exit_status.success() {
        Ok(())
    } else {
        Err(exit_status.code().unwrap_or(-1))
    }
}

#[must_use]
pub fn help_message() -> &'static str {
    color_print::cstr!(
"Checks a package to catch common mistakes and improve your Rust code.

<green,bold>Usage</>:
    <cyan,bold>cargo crushy</> <cyan>[OPTIONS] [--] [<<ARGS>>...]</>

<green,bold>Common options:</>
    <cyan,bold>--no-deps</>                Run Crushy only on the given crate, without linting the dependencies
    <cyan,bold>--fix</>                    Automatically apply lint suggestions. This flag implies <cyan>--no-deps</> and <cyan>--all-targets</>
    <cyan,bold>-h</>, <cyan,bold>--help</>               Print this message
    <cyan,bold>-V</>, <cyan,bold>--version</>            Print version info and exit
    <cyan,bold>--explain [LINT]</>         Print the documentation for a given lint

See all options with <cyan,bold>cargo check --help</>.

<green,bold>Allowing / Denying lints</>

To allow or deny a lint from the command line you can use <cyan,bold>cargo crushy --</> with:

    <cyan,bold>-W</> / <cyan,bold>--warn</> <cyan>[LINT]</>       Set lint warnings
    <cyan,bold>-A</> / <cyan,bold>--allow</> <cyan>[LINT]</>      Set lint allowed
    <cyan,bold>-D</> / <cyan,bold>--deny</> <cyan>[LINT]</>       Set lint denied
    <cyan,bold>-F</> / <cyan,bold>--forbid</> <cyan>[LINT]</>     Set lint forbidden

You can use tool lints to allow or deny lints from your code, e.g.:

    <yellow,bold>#[allow(crushy::needless_lifetimes)]</>

<green,bold>Manifest Options:</>
    <cyan,bold>--manifest-path</> <cyan><<PATH>></>  Path to Cargo.toml
    <cyan,bold>--frozen</>                Require Cargo.lock and cache are up to date
    <cyan,bold>--locked</>                Require Cargo.lock is up to date
    <cyan,bold>--offline</>               Run without accessing the network
")
}
#[cfg(test)]
mod tests {
    use super::CrushyCmd;

    #[test]
    fn fix() {
        let args = "cargo crushy --fix".split_whitespace().map(ToString::to_string);
        let cmd = CrushyCmd::new(args);
        assert_eq!("fix", cmd.cargo_subcommand);
        assert!(!cmd.args.iter().any(|arg| arg.ends_with("unstable-options")));
    }

    #[test]
    fn fix_implies_no_deps() {
        let args = "cargo crushy --fix".split_whitespace().map(ToString::to_string);
        let cmd = CrushyCmd::new(args);
        assert!(cmd.crushy_args.iter().any(|arg| arg == "--no-deps"));
    }

    #[test]
    fn no_deps_not_duplicated_with_fix() {
        let args = "cargo crushy --fix -- --no-deps"
            .split_whitespace()
            .map(ToString::to_string);
        let cmd = CrushyCmd::new(args);
        assert_eq!(cmd.crushy_args.iter().filter(|arg| *arg == "--no-deps").count(), 1);
    }

    #[test]
    fn check() {
        let args = "cargo crushy".split_whitespace().map(ToString::to_string);
        let cmd = CrushyCmd::new(args);
        assert_eq!("check", cmd.cargo_subcommand);
    }
}
