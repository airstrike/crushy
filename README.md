# Crushy

iced-specific Rust lints. A fork of [rust-clippy](https://github.com/rust-lang/rust-clippy) with every upstream lint removed and a new set written from scratch for [iced](https://github.com/iced-rs/iced) projects.

Clippy nudges. Crushy crushes.

## Build

Requires a nightly toolchain (pinned in `rust-toolchain.toml`).

```sh
cargo build --release
```

## Run

```sh
PATH=$(pwd)/target/release:$PATH cargo crushy
```

## License

MIT OR Apache-2.0.
