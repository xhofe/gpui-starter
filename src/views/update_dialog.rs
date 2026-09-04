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

//! Action area of the "update available" dialog.

use crate::states::{GlobalEvent, GlobalStore, i18n_update};
use gpui::{App, Subscription, Window, prelude::*};
use gpui_kit::component::{
    ActiveTheme, WindowExt,
    button::{Button, ButtonVariants},
    h_flex,
    label::Label,
    progress::Progress,
    v_flex,
};
use humansize::{DECIMAL, format_size};
use std::rc::Rc;
use tracing::debug;

pub type DialogCallback = Rc<dyn Fn(&mut Window, &mut App)>;

pub struct UpdateDialog {
    on_download: DialogCallback,
    on_skip: DialogCallback,
    was_downloading: bool,
    _subscriptions: Vec<Subscription>,
}

impl UpdateDialog {
    pub fn new(on_download: DialogCallback, on_skip: DialogCallback, cx: &mut Context<Self>) -> Self {
        let global_state = cx.global::<GlobalStore>().state();
        let subscription = cx.subscribe(&global_state, |_this, _state, event, cx| {
            if matches!(event, GlobalEvent::UpdateDownloadProgress) {
                cx.notify();
            }
        });

        Self {
            on_download,
            on_skip,
            was_downloading: false,
            _subscriptions: vec![subscription],
        }
    }

    fn render_actions(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let on_download = self.on_download.clone();
        let on_skip = self.on_skip.clone();
        h_flex()
            .w_full()
            .justify_end()
            .gap_2()
            .child(
                Button::new("update-skip")
                    .outline()
                    .label(i18n_update(cx, "skip_version"))
                    .on_click(move |_, window, cx| {
                        on_skip(window, cx);
                        window.close_dialog(cx);
                    }),
            )
            .child(
                Button::new("update-download")
                    .primary()
                    .label(i18n_update(cx, "download"))
                    .on_click(move |_, window, cx| on_download(window, cx)),
            )
    }

    fn render_restart(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        v_flex()
            .w_full()
            .gap_2()
            .child(
                Label::new(i18n_update(cx, "restart_body"))
                    .text_sm()
                    .text_color(muted)
                    .whitespace_normal(),
            )
            .child(
                h_flex()
                    .w_full()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("update-restart-later")
                            .outline()
                            .label(i18n_update(cx, "restart_later"))
                            .on_click(|_, window, cx| {
                                cx.global::<GlobalStore>().clone().update(cx, |state, cx| {
                                    state.set_update_installed(false, cx);
                                });
                                window.close_dialog(cx);
                            }),
                    )
                    .child(
                        Button::new("update-restart-now")
                            .primary()
                            .label(i18n_update(cx, "restart_now"))
                            .on_click(|_, _window, cx| {
                                debug!("update: restarting into the freshly installed bundle");
                                #[cfg(target_os = "macos")]
                                crate::helpers::relaunch();
                                cx.quit();
                            }),
                    ),
            )
    }

    fn render_progress(&self, done: u64, total: u64, cx: &mut Context<Self>) -> impl IntoElement {
        let pct = (done * 100).checked_div(total).unwrap_or(0).min(100);
        let muted = cx.theme().muted_foreground;
        v_flex()
            .w_full()
            .gap_1p5()
            .child(Progress::new("update-progress").value(pct as f32))
            .child(
                h_flex()
                    .w_full()
                    .justify_between()
                    .child(Label::new(format!("{} · {pct}%", i18n_update(cx, "downloading"))).text_sm())
                    .child(
                        Label::new(format!(
                            "{} / {}",
                            format_size(done, DECIMAL),
                            format_size(total, DECIMAL)
                        ))
                        .text_sm()
                        .text_color(muted),
                    ),
            )
    }
}

impl Render for UpdateDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (progress, installed) = {
            let store = cx.global::<GlobalStore>().read(cx);
            (store.download_progress(), store.update_installed())
        };
        if progress.is_some() {
            self.was_downloading = true;
        } else if self.was_downloading && !installed {
            self.was_downloading = false;
            debug!("update dialog: download settled, closing");
            cx.defer_in(window, |_this, window, cx| window.close_dialog(cx));
        }

        match progress {
            Some((done, total)) => self.render_progress(done, total, cx).into_any_element(),
            None if installed => self.render_restart(cx).into_any_element(),
            None => self.render_actions(cx).into_any_element(),
        }
    }
}
