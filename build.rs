fn main() {
    // Forward the profile to the main compilation
    println!("cargo:rustc-env=PROFILE={}", std::env::var("PROFILE").unwrap());
    // Don't rebuild even if nothing changed
    println!("cargo:rerun-if-changed=build.rs");
    bake_toolchain_identity();
    rustc_tools_util::setup_version_info!();
}

/// Bake the toolchain crushy is built against so `cargo-crushy` can force a
/// matching toolchain on consumers at runtime (see `.claude/DECISIONS.md`, D1).
fn bake_toolchain_identity() {
    // The pinned toolchain *name*, read verbatim from rust-toolchain.toml.
    // Don't reconstruct it from `rustc -vV`'s commit-date: the toolchain
    // `nightly-2026-05-30` reports commit-date `2026-05-29` (off-by-one).
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let toml_path = format!("{manifest}/rust-toolchain.toml");
    let toml = std::fs::read_to_string(&toml_path).expect("read rust-toolchain.toml");
    let channel = toml
        .lines()
        .find_map(|l| l.trim().strip_prefix("channel"))
        .and_then(|rest| rest.trim_start().strip_prefix('='))
        .map(|v| v.trim().trim_matches('"'))
        .expect("`channel = \"...\"` in rust-toolchain.toml");
    println!("cargo:rustc-env=CRUSHY_NIGHTLY={channel}");
    println!("cargo:rerun-if-changed=rust-toolchain.toml");

    // The rustc commit-hash this build is ABI-locked to. Compared at runtime
    // against the consumer's active toolchain to decide whether to override.
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let out = std::process::Command::new(rustc)
        .arg("-vV")
        .output()
        .expect("run `rustc -vV`");
    let text = String::from_utf8(out.stdout).expect("`rustc -vV` is utf8");
    let commit = text
        .lines()
        .find_map(|l| l.strip_prefix("commit-hash:"))
        .map(str::trim)
        .expect("`commit-hash:` in `rustc -vV`");
    println!("cargo:rustc-env=CRUSHY_RUSTC_COMMIT={commit}");
}
