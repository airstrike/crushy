# Crushy

**An [iced](https://github.com/iced-rs/iced)-specific linter for Rust.**

Crushy is a fork of [rust-clippy](https://github.com/rust-lang/rust-clippy) with every upstream lint removed and a new set written from scratch to catch patterns specific to iced application code. It reuses Clippy's driver machinery, so lints are configured with ordinary `#[allow]` / `#[warn]` attributes and require no setup in the crates you run it against.

## Requirements

Crushy links against `rustc`'s private internals, which carry no stability guarantee and change between nightlies. The repository pins a specific toolchain in `rust-toolchain.toml` (currently `nightly-2026-05-13`), and **any crate you lint must be built with that same toolchain.** A mismatch fails at load time, when the dynamic linker cannot find the exact `librustc_driver` build Crushy was compiled against.

## Installation

```sh
git clone https://github.com/airstrike/crushy
cd crushy
cargo install --path .
```

This builds `cargo-crushy` and `crushy-driver` against the pinned toolchain and installs both to `~/.cargo/bin`.

Do **not** pass `+nightly`: an explicit toolchain overrides the pin in `rust-toolchain.toml`, and the rolling `nightly` channel usually lacks the `rustc-dev` component the build needs. After a `rustup` update that advances the pin, reinstall with `cargo install --path . --force`.

## Usage

Run it from any iced project pinned to the same nightly:

```sh
cargo crushy
```

Configure individual lints with standard attributes, and read a lint's full rationale with `--explain`:

```rust
#![warn(crushy::use_as_rename)]
#![allow(crushy::length_fill)]
```

```sh
cargo crushy --explain deep_path
```

## Lints

| Lint | Category | Default | Description |
|---|---|---|---|
| `crushy::deep_path` | style | warn | Inline paths with four or more segments (e.g. `a::b::c::d()`); bring the item into scope with `use`. |
| `crushy::length_fill` | style | warn | `Length::Fill` and `Length::Fixed(_)`; prefer `iced::Fill` or a bare number. |
| `crushy::use_as_rename` | restriction | allow | `use ... as Name` import aliases (`as _` and `as self` are exempt). |

## Lint levels

Crushy keeps Clippy's category-to-level mapping; the category a lint is filed under determines its default level.

| Category | Default level |
|---|---|
| `correctness` | deny |
| `style`, `complexity`, `perf`, `suspicious` | warn |
| `pedantic`, `restriction`, `nursery`, `cargo` | allow (opt-in) |

## How it works

`crushy-driver` is a thin wrapper around `rustc`. Running `cargo crushy` re-invokes Cargo with `RUSTC_WORKSPACE_WRAPPER` pointed at the driver, which registers Crushy's lint passes and injects `#![feature(register_tool)] #![register_tool(crushy)]` into every crate it compiles (via `-Z crate-attr`). That injection is why `#[allow(crushy::...)]` resolves in your code without any manual configuration.

## Writing a lint

A lint is wired up in three places:

1. `crushy_lints/src/<name>.rs` — the implementation. `length_fill.rs` is a minimal example: `declare_crushy_lint!` declares the lint, `declare_lint_pass!` declares the pass, and `impl EarlyLintPass` performs the match.
2. `crushy_lints/src/lib.rs` — register the pass in `register_lint_passes`.
3. `crushy_lints/src/declared_lints.rs` — add the lint's `_INFO` entry to the `LINTS` array.

Use `LateLintPass` instead of `EarlyLintPass` when a lint needs type information; the type-inspection helpers live in `crushy_utils`.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT), at your option.

## Acknowledgments

Crushy is built on rust-clippy's substrate: `clippy_utils` (renamed `crushy_utils`), the `declare_clippy_lint!` macro (renamed `declare_crushy_lint!`), the `cargo dev` tooling, and the driver wrapper. The lint implementations are original.
