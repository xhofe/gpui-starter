# Contributing

Thanks for your interest in improving this starter. This guide covers how to get set up and what we look for in a contribution.

## Setup

The app is built with Rust (edition 2024; **Rust 1.98.0** is the toolchain we build with — pinned in `rust-toolchain.toml`, which rustup applies automatically) and [GPUI](https://www.gpui.rs/).

```bash
git clone https://github.com/xhofe/gpui-starter
cd gpui-starter
make dev
```

Before a PR: `make fmt && make lint && make test`.

## Conventions

- **Components** — prefer `gpui-component`'s built-in components first; fall back to the shared widgets in `crates/gpui-starter-ui` only when none fit.
- **Prefs vs app data** — preferences go in `gpui-starter.toml` (`AppState`); durable rows go in `gpui-starter-db` (redb) with `#[serde(default)]` and skip-unreadable-row loaders.
- **Locales** — every UI string lives in both `locales/en.toml` and `locales/zh.toml` with the same key set.
- Open an issue before a large feature so we can align on scope.

Please read and agree to the [Contributor License Agreement](CLA.md).
