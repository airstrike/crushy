//! Resolve which rustc toolchain `cargo crushy` should run the check under.
//!
//! `crushy-driver` is dynamically linked against one specific `librustc_driver`
//! and is therefore ABI-locked to the exact rustc it was built with. The crate
//! being linted must be checked under that same toolchain or dyld fails to load
//! the driver.
//!
//! We match by the rustc *commit-hash* baked at build time, not by toolchain
//! *name*: a custom-linked toolchain (e.g. `dev`) is just a sysroot of symlinks
//! and can wrap any rustc, so the name says nothing about ABI compatibility.
//! See `.claude/DECISIONS.md`, D1.

use std::process::Command;

/// The toolchain crushy was built against, e.g. `nightly-2026-05-30` (verbatim
/// from `rust-toolchain.toml`, baked by `build.rs`).
const PINNED_NIGHTLY: &str = env!("CRUSHY_NIGHTLY");
/// The rustc commit-hash `crushy-driver` is ABI-compatible with.
const PINNED_COMMIT: &str = env!("CRUSHY_RUSTC_COMMIT");

/// The toolchain to run the check under, or `None` to leave the consumer's
/// active toolchain untouched.
///
/// We decide on rustc *commit-hash*, not toolchain name: cargo always injects
/// `RUSTUP_TOOLCHAIN` (its resolved toolchain) into subcommands, so that env
/// var can't tell us the consumer's intent. Returns `None` only when the active
/// toolchain's rustc is already the one crushy was built against (ABI-compatible
/// — lint under the consumer's own toolchain). Otherwise force crushy's nightly.
///
/// Escape hatch: `CRUSHY_TOOLCHAIN=<name>` forces a specific toolchain (e.g. a
/// crushy built against a custom one); `CRUSHY_TOOLCHAIN=` (empty) disables
/// forcing entirely.
pub fn force() -> Option<String> {
    if let Some(tk) = std::env::var_os("CRUSHY_TOOLCHAIN") {
        let tk = tk.to_string_lossy().into_owned();
        return (!tk.is_empty()).then_some(tk);
    }

    if active_rustc_commit().as_deref() == Some(PINNED_COMMIT) {
        return None;
    }

    Some(PINNED_NIGHTLY.to_owned())
}

/// The toolchain crushy was built against, e.g. `nightly-2026-05-30`. Used to
/// run `crushy-driver` standalone (e.g. for `--explain`) under a matching
/// toolchain via `rustup run`.
pub fn pinned_nightly() -> &'static str {
    PINNED_NIGHTLY
}

/// The `commit-hash` from the active toolchain's `rustc -vV`, if determinable.
///
/// A bare `rustc` hits the rustup proxy, which resolves the consumer's active
/// toolchain from the current directory. Returns `None` on any failure or when
/// the hash is `unknown` (custom local builds) — callers then force the pinned
/// toolchain, which is the ABI-safe default.
fn active_rustc_commit() -> Option<String> {
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let out = Command::new(rustc).arg("-vV").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    let hash = text.lines().find_map(|l| l.strip_prefix("commit-hash:"))?.trim();
    (hash != "unknown").then(|| hash.to_owned())
}
