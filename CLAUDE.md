# CLAUDE.md

Guidance for coding agents working in this repository.

This is a native desktop app starter built in Rust with [GPUI](https://www.gpui.rs/) and `gpui-component`, both taken from crates.io through `gpui-kit`. GPUI is the `gpui-pre-*` snapshot family gpui-kit pins (renamed back to `gpui` / `gpui_platform` / `gpui_macros` in `[workspace.dependencies]`, so source keeps `gpui::…`). Components and icon assets are `gpui_kit::component::…` / `gpui_kit::assets::Assets`. Bump the four together and confirm one `gpui-pre` in `Cargo.lock`.

Placeholder identity (before `./scripts/init.sh my-app`): display **GPUI Starter**, kebab/bin/`APP_ID` `gpui-starter`, snake `gpui_starter`, env `GPUI_STARTER_*`, bundle `com.example.gpui-starter`. Application types stay generic (`AppState`, `GlobalStore`, `Card`) so init does not have to rename them.

## Commands

- Build / typecheck: `cargo check`
- Lint: **run `make lint` once as the final step before completing any work** (and after every change) — it is the required gate and runs `typos` + `cargo clippy --all-targets --all -- --deny=warnings`. Never report work as done until `make lint` passes clean. `cargo clippy --tests -- -D warnings` alone is *not* enough: it skips `typos`.
- Format: **run `make fmt` (`cargo fmt`) after every code change**, before the final `make lint`.
- Dependency gate: `make deny` (`cargo deny check`, config in `deny.toml`).
- Tests: `make test` (`cargo test --workspace`).
- Locale hygiene: `make check-locales` (`tests/locale_keys.rs`, also part of `make test`). Locales are `en` and `zh`; `build.rs` panics if their key sets drift.
- Run dev: `make dev` (`bacon run`); with logs: `make debug` (`RUST_LOG=DEBUG`).
- Release: `make release` (`cargo build --release --features mimalloc`).
- Toolchain: Rust **1.98.0**, edition 2024 — pinned in `rust-toolchain.toml`. The published MSRV is `rust-version` in `Cargo.toml`.

Clippy `unwrap_used = "deny"` is set crate-wide **including tests** — use `.expect("…")` or proper matching in test code, never `.unwrap()`.

**Avoid `#[allow(clippy::…)]` and `#[allow(dead_code)]`.** Prefer fixing the underlying smell. Dead code: **delete it or wire it up**. Reach for `allow` only as a last resort when the lint is a genuine false positive or an unavoidable external-API constraint, and keep the attribute as narrow as possible with a one-line comment explaining why.

## Workspace layout

Cargo workspace: root binary crate `gpui-starter` (bin name `gpui-starter`) plus `members = ["crates/*"]`. Shared dependency versions live in the root `[workspace.dependencies]`.

- `crates/gpui-starter-ui` — reusable widgets (`Card`, `Dialog`, `Form`, `Select`, `TextTable`, …). **Separate crate**: it cannot use `crate::helpers::*` from the app. Platform-specific values (e.g. monospace font family) and localized strings must be passed in by the caller.
- `crates/gpui-starter-db` — redb-backed local storage. Every value struct carries container-level `#[serde(default)]` (adding a field must never make an existing row unreadable). A loader **skips** a row it cannot read, never deletes it. A new `TableDefinition` must also be opened in `ensure_schema`. The demo table is `todos`.

Preferences persist to `gpui-starter.toml` via `update_app_state_and_save`. App data that belongs in redb goes in `gpui-starter-db`.

## Architecture

**State (`src/states/`)** — the source of truth, GPUI entities.

- `GlobalStore` / `AppState` (`app.rs`): app-wide config. Persisted to `gpui-starter.toml` via `update_app_state_and_save(cx, "action", |state, _| …)` (async, debounced). Add a field + getter/setter here to persist a new preference.
- Events: `GlobalEvent` (notifications, `RouteChanged`, update progress) drives view updates via `cx.subscribe`.
- i18n: `t!("section.key")` (rust-i18n). Use the `i18n_<section>(cx, key)` helpers in `states/i18n.rs`, each **individually** re-exported from `states.rs`.

**Views (`src/views/`)** — one GPUI view per route/panel. `content.rs` is the route switcher. `root.rs`'s `AppRoot` holds sidebar + workspace tabs (`Vec<ContentTab>`, one `Content` per tab; only the active tab reacts to global route broadcasts) + title bar, and registers global `.on_action` handlers. `main.rs` is the entry point; `dialogs.rs` holds app-level dialogs (crash, welcome, update); `window_setup.rs` window placement + theme application; `startup.rs` CLI flags, smoke gates and the database recovery window.

**Timestamps:** every user-facing date / time goes through `helpers/datetime.rs` (`format_unix_secs`, `now_datetime`, …), which applies the Settings time-zone and date-layout preference from a process-wide slot (`set_datetime_prefs`). Never call `Local::now().format(…)` in a view for display — file-name stamps and diagnostics are the only fixed-format exceptions.

**Keybindings:** user-configurable shortcuts are the `HOT_KEYS` table in `helpers/action.rs`. A new user-visible shortcut is one table row, never a second hand-written list. Overrides live in `<config_dir>/keybindings.toml` (restart to apply).

**Single instance:** `claim_instance` (before `init_database`) forwards a second launch to the running process over a loopback socket + token (`helpers/single_instance.rs`).

**Diagnostics:** title-bar menu → Export Diagnostics writes `gpui-starter-diagnostics-<stamp>.zip` via `helpers/diagnostics.rs`. Add new secrets to `AppState::redacted_toml`.

**Smoke mode:** `GPUI_STARTER_SMOKE_TEST=1` exits 0 on the first painted frame; `GPUI_STARTER_SMOKE_GATE=window` accepts "window created + 5s alive".

**Imports:** bring items into scope with `use` declarations at the top of the file. Do **not** write fully-qualified paths inline except to disambiguate two same-named types or a single use inside a macro.

## GPUI gotchas

The general GPUI / gpui-component rules live in the `gpui-kit` and `gpui-kit-design-guides` skills. This list is what this codebase hit on top of them.

- `gpui_kit::init(cx)` in the skills is `gpui_kit::component::init(cx)` in `main.rs`.
- The skills write `use gpui_kit::*;` for GPUI. Here GPUI stays `gpui::…`.
- `gpui_kit::component::list::ListItem` impls `InteractiveElement`; listeners registered through those traits are **not** gated by `disabled`/`separator`, and the hover style is managed internally — use `.on_hover(...)`, not `.hover(...)`.
- **Text input is three types:** `InputState`/`Input` (single line), `TextareaState`/`Textarea`, `EditorState`/`Editor`. `folding` **defaults to true** on Editor and reserves a fold-icon hitbox even with line numbers off.
- **`Scrollable` + `max_h` silently clips instead of scrolling.** Give the viewport a definite `h(...)` — never `max_h` with `overflow_y_scrollbar()`. Native `overflow_y_scroll()` + `track_scroll` is the pattern for adaptive-height lists.
- `TabBar::child(impl Into<Tab>)` only accepts `Tab`, so a context menu or drag & drop can't be attached to its tabs. The workspace tab strip is hand-rolled for that reason.
- Clippy's `should_implement_trait` fires on `pub fn from_str` in **lib crates** — name such constructors `from_name`.
- `InputState` placeholder / default-value strings must **not** contain `\n`.
- **A dialog body that holds an `Input` must be a view entity, not inline elements** — `.child(move || some_view.clone())`. `Dialog::child` is an `Fn() -> AnyElement` the dialog re-invokes every frame; hanging the input off that rebuilt tree plus a sibling `flex_wrap()` corrupts the field.
- `Dialog` builds its own footer: a non-alert dialog renders **no buttons at all** unless `.ok_text(…)` (or `.footer_child(…)`) is set on the dialog itself.
- `IconName` (gpui-component) is a fixed enum — verify a variant exists before using it.
- Theme-derived colors must be read before a `move` render closure (can't borrow `cx` inside).
- **`Input`/`NumberInput` balloon inside grid cards — pin their height.** Give text inputs an explicit `.h(px(32.))`.
- **`Button.loading(true)` shows no spinner without an icon.**
- **A bare `div()` is `display: Block` — flex sizing on its children silently dies there.** Any wrapper on the path from a sized ancestor down to a `uniform_list`/`h_full` consumer must be a flex container (`v_flex().flex_1().min_h_0()`).
- **`Label` ignores a parent's `.text_color()`.** Set `.text_color(...)` on each `Label` itself.
- **Bold needs a concrete font family.** `.font_weight(FontWeight::BOLD)` alone often looks unchanged on the system UI font. Use `.font_family(get_mono_font_family())` (JetBrains Mono is bundled).

## Conventions

- UI components: **prefer `gpui-component`'s built-in components first** — the `gpui-kit` skill's component catalog is the list to check. Only when `gpui-component` has no suitable component, use `crates/gpui-starter-ui`.
- Destructive ops route through a confirm dialog (`Dialog::new_alert` + `dialog_button_props`).
- Keep the dependency surface lean.
- Comments in the code always use English.

## Agent skills

Two upstream skills are vendored in `.claude/skills/` (from `longbridge/gpui-kit`):

- `gpui-kit` — load before any GPUI or gpui-component work. `references/coding-guides.md` is the normative coding guide.
- `gpui-kit-design-guides` — load before changing anything with a visible surface.

How the skills' code maps onto this repo: GPUI stays `gpui::…`; `gpui_kit::init(cx)` is `gpui_kit::component::init(cx)`; `#[gpui::test]` not `#[gpui_kit::test]`.
