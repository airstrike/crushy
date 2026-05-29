# Crushy

iced-specific Rust lints. A fork of [rust-clippy](https://github.com/rust-lang/rust-clippy) with every upstream lint removed and a new set written from scratch for [iced](https://github.com/iced-rs/iced) projects.

Clippy nudges. Crushy crushes.

## Install

Requires nightly (`rust-toolchain.toml` pins `nightly-2026-05-13`; consumer crates must match).

```sh
git clone https://github.com/airstrike/crushy ~/projects/crushy
cd ~/projects/crushy
cargo build --release
```

## Usage

From any iced project pinned to the same nightly:

```sh
PATH=~/projects/crushy/target/release:$PATH cargo crushy
```

Configure individual lints with standard rustc attributes:

```rust
#![warn(crushy::use_as_rename)]
#![allow(crushy::length_fill)]
```

## Lints

| Lint | Category | Default | What it flags |
|---|---|---|---|
| `crushy::deep_path` | style | warn | inline paths with 4+ segments (e.g. `a::b::c::d()`) — bring the item into scope with `use` |
| `crushy::length_fill` | style | warn | `Length::Fill` and `Length::Fixed(_)` — use `iced::Fill` or bare numbers |
| `crushy::use_as_rename` | restriction | allow | `use ... as Name` import aliases (`as _` and `as self` exempt) |

## Categories

Inherited from clippy.

| Category | Default |
|---|---|
| `crushy::correctness` | deny |
| `crushy::style` / `complexity` / `perf` / `suspicious` | warn |
| `crushy::pedantic` / `restriction` / `nursery` / `cargo` | allow (opt-in) |

## Writing a lint

`crushy_lints/src/length_fill.rs` is the minimal exemplar: `declare_crushy_lint! { ... }` declares the lint, `declare_lint_pass!` declares the pass, `impl EarlyLintPass` does the matching. Register the pass in `crushy_lints/src/lib.rs::register_lint_passes` and add `_INFO` to `declared_lints::LINTS`.

Use `LateLintPass` instead when the lint needs type information; the type APIs come from `crushy_utils`.

## How it works

`crushy-driver` wraps rustc. The driver injects `#![feature(register_tool)] #![register_tool(crushy)]` into every crate it compiles (via `-Z crate-attr`), so `#[allow(crushy::...)]` resolves without any setup in consumer crates.

## License

MIT OR Apache-2.0.

## Acknowledgments

The substrate is [rust-clippy](https://github.com/rust-lang/rust-clippy): `clippy_utils` (renamed `crushy_utils`), the `declare_clippy_lint!` macro (renamed `declare_crushy_lint!`), the dev tooling (`crushy_dev`), and the driver wrapper.
