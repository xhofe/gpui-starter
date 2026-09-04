# GPUI Starter

A native desktop app starter built in Rust with [GPUI](https://www.gpui.rs/) and [gpui-kit](https://github.com/longbridge/gpui-kit). Clone it, rename it, and start building.

The placeholder identity is **GPUI Starter** (`gpui-starter`). After you clone this repo (or use it as a GitHub Template), run:

```bash
./scripts/init.sh my-app
```

That rewrites crate names, bundle ids, CI, locales, and docs from `origin` + your git user. Preferences persist to `my-app.toml`; the todos demo uses a local redb file.

## Run

Rust **1.98.0** is pinned in `rust-toolchain.toml` (rustup applies it). Then:

```bash
make dev          # bacon run
make debug        # RUST_LOG=DEBUG
make test
make fmt && make lint
```

`make lint` is the repo gate: `typos` + `cargo clippy --all-targets --all -- --deny=warnings`.

## Layout

```
src/                      # bin crate `gpui-starter`
  main.rs, root.rs, …
  states/app.rs           # prefs: theme / locale / fonts / proxy / update / tray / datetime / window
  views/{home,todos,settings,about,title_bar,sidebar,…}
crates/gpui-starter-ui/   # Card, Dialog, Form, Select, TextTable, …
crates/gpui-starter-db/   # redb `todos` table
locales/{en,zh}.toml
scripts/init.sh
```

Sidebar routes: **Home** | **Todos** | **Settings**. About, updates, the command palette, and keyboard shortcuts live on the title bar / palette. Workspace tabs are independent shells; todos and prefs are global.

## Release

`.github/workflows/publish.yml` builds macOS / Windows / Linux (deb, rpm, AppImage, tarball, MSI). Smoke, lint, audit, and udeps stay in CI.

## License

Apache-2.0. See [LICENSE](./LICENSE).
