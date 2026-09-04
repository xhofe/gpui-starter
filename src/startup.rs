// Copyright 2026 Andy Hsu.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Pre-window startup pieces: version constants, the database recovery
//! window, and the smoke-test gates.

use crate::helpers::get_or_create_config_dir;
use crate::launch;
use crate::states::AppState;
use gpui::{SharedString, Window, div, prelude::*};
use gpui_kit::component::{
    ActiveTheme, StyledExt,
    button::{Button, ButtonVariants},
    h_flex,
    label::Label,
    v_flex,
};
use gpui_starter_db::{DbOpenFailure, init_database, quarantine_database};
use rust_i18n::t;
use std::path::PathBuf;
use tracing::{error, warn};

pub(crate) const VERSION: &str = env!("CARGO_PKG_VERSION");
pub(crate) const GIT_SHA: &str = env!("VERGEN_GIT_SHA");
pub(crate) const BUILD_TIMESTAMP: &str = env!("VERGEN_BUILD_TIMESTAMP");
pub(crate) const BUILD_CHANNEL: &str = match option_env!("GPUI_STARTER_BUILD_CHANNEL") {
    Some(channel) => channel,
    None => "stable",
};

pub(crate) fn is_nightly_build() -> bool {
    BUILD_CHANNEL == "nightly"
}

pub(crate) fn database_path() -> std::io::Result<PathBuf> {
    Ok(get_or_create_config_dir()?.join("gpui-starter.redb"))
}

pub(crate) struct DatabaseErrorView {
    failure: DbOpenFailure,
    app_state: AppState,
    rebuild_error: Option<String>,
}

impl DatabaseErrorView {
    pub(crate) fn new(failure: DbOpenFailure, app_state: AppState) -> Self {
        Self {
            failure,
            app_state,
            rebuild_error: None,
        }
    }

    fn text(&self, key: &str) -> SharedString {
        t!(format!("database.{key}"), locale = self.app_state.locale())
            .to_string()
            .into()
    }

    fn rebuild(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let path = match database_path() {
            Ok(path) => path,
            Err(e) => {
                self.rebuild_error = Some(e.to_string());
                cx.notify();
                return;
            }
        };
        let quarantined = match quarantine_database(&path) {
            Ok(path) => path,
            Err(e) => {
                error!(error = %e, "could not move the local database aside");
                self.rebuild_error = Some(e.to_string());
                cx.notify();
                return;
            }
        };
        warn!(quarantined = %quarantined.display(), "local database moved aside; creating a fresh one");
        if let Err(e) = init_database(&path) {
            error!(error = %e, "rebuilding the local database failed");
            self.rebuild_error = Some(e.to_string());
            cx.notify();
            return;
        }
        let handle = window.window_handle();
        launch(cx, self.app_state.clone());
        cx.spawn(async move |_this, cx| {
            cx.update(|cx| {
                if cx.windows().len() > 1 {
                    let _ = handle.update(cx, |_, window, _| window.remove_window());
                }
            });
        })
        .detach();
    }
}

impl Render for DatabaseErrorView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (body_key, can_rebuild) = match &self.failure {
            DbOpenFailure::Locked => ("locked_body", false),
            DbOpenFailure::SchemaTooNew { .. } => ("schema_too_new_body", true),
            DbOpenFailure::Damaged(_) => ("damaged_body", true),
            DbOpenFailure::Inaccessible(_) => ("inaccessible_body", false),
        };
        let detail: Option<String> = match &self.failure {
            DbOpenFailure::Locked => None,
            DbOpenFailure::SchemaTooNew { found, supported } => Some(format!("schema v{found} > v{supported}")),
            DbOpenFailure::Damaged(message) | DbOpenFailure::Inaccessible(message) => Some(message.clone()),
        };
        let rebuild_error = self
            .rebuild_error
            .as_ref()
            .map(|e| format!("{}: {e}", self.text("rebuild_failed")));
        let (title, body, quit, rebuild) = (
            self.text("title"),
            self.text(body_key),
            self.text("quit"),
            self.text("rebuild"),
        );
        let muted = cx.theme().muted_foreground;
        let danger = cx.theme().danger;
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .p_5()
            .gap_3()
            .child(Label::new(title).font_semibold())
            .child(Label::new(body).whitespace_normal())
            .when_some(detail, |this, detail| {
                this.child(Label::new(detail).text_xs().text_color(muted).whitespace_normal())
            })
            .when_some(rebuild_error, |this, message| {
                this.child(Label::new(message).text_color(danger).whitespace_normal())
            })
            .child(div().flex_1())
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("quit-db-error")
                            .label(quit)
                            .on_click(|_, _window, cx| cx.quit()),
                    )
                    .when(can_rebuild, |this| {
                        this.child(
                            Button::new("rebuild-db")
                                .label(rebuild)
                                .primary()
                                .on_click(cx.listener(|this, _, window, cx| this.rebuild(window, cx))),
                        )
                    }),
            )
    }
}

pub(crate) fn is_smoke_test() -> bool {
    std::env::var("GPUI_STARTER_SMOKE_TEST").is_ok_and(|v| v == "1")
}

pub(crate) fn smoke_gate_is_window() -> bool {
    std::env::var("GPUI_STARTER_SMOKE_GATE").is_ok_and(|v| v == "window")
}
