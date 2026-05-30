# Contributing to Crushy

## Development setup

Crushy builds against the nightly pinned in `rust-toolchain.toml`; `rustup` selects it automatically inside this directory. Enable the git hooks once per clone:

```sh
git config core.hooksPath scripts/hooks
```

- **pre-commit** — `cargo fmt --all --check` and `cargo clippy --workspace -- -D warnings` on staged Rust, blocks newly-added `mod.rs`, and restricts root-level `.md`.
- **pre-push** — blocks AI attribution in commit history, runs `cargo audit` and `cargo fmt --all --check`.

`cargo audit` requires `cargo install cargo-audit`.

## Building

```sh
cargo build --release   # cargo-crushy + crushy-driver
```

## Adding a lint

`cargo dev new_lint` scaffolds the files; see [Writing a lint](README.md#writing-a-lint) for the three-place wire-up.

## Bumping the pinned nightly

`master` tracks the latest supported nightly, and each previous pin is preserved as a `nightly-YYYY-MM-DD` git tag. Advance it with `scripts/bump-nightly.sh`, or let the weekly **Bump nightly** workflow open a pre-verified PR.
