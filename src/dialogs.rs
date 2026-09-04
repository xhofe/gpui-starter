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

//! App-level dialogs opened from the root: crash report, first-run welcome,
//! and update / install prompts.

use crate::helpers::{
    ConfigRecovery, CrashReport, UpdateInfo, focus_installer_ui, get_mono_font_family, humanize_keystroke, logs_dir,
};
use crate::root::AppRoot;
use crate::states::{GlobalStore, i18n_crash, i18n_hints, i18n_update, update_app_state_and_save_quiet};
use crate::views::{DialogCallback, UpdateDialog};
use gpui::{App, SharedString, WeakEntity, Window, div, prelude::*, px, rems};
use gpui_kit::component::{
    ActiveTheme, IconName,
    label::Label,
    scroll::ScrollableElement,
    text::{TextView, TextViewStyle},
    v_flex,
};
use gpui_starter_ui::Dialog;
use rust_i18n::t;
use std::{cell::Cell, rc::Rc};
use tracing::{error, info};

pub(crate) fn release_notes_style() -> TextViewStyle {
    TextViewStyle::default()
        .paragraph_gap(rems(0.5))
        .heading_font_size(|level, _base| match level {
            1 => px(18.),
            2 => px(16.),
            3 => px(15.),
            _ => px(14.),
        })
}

pub(crate) fn config_recovery_message(recovery: &ConfigRecovery, cx: &App) -> SharedString {
    let locale = cx.global::<GlobalStore>().read(cx).locale();
    let file = recovery
        .path()
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let corrupt = recovery.corrupt_path().display().to_string();
    let key = match recovery {
        ConfigRecovery::RestoredFromBackup { .. } => "common.config_restored_from_backup",
        ConfigRecovery::Reset { .. } => "common.config_reset",
    };
    t!(key, file = file, corrupt = corrupt, locale = locale)
        .to_string()
        .into()
}

pub(crate) fn open_crash_dialog(report: &CrashReport, window: &mut Window, cx: &mut App) {
    let locale = cx.global::<GlobalStore>().read(cx).locale().to_string();
    let body = i18n_crash(cx, "body");
    let summary: SharedString = report.summary.clone().into();
    let saved: SharedString = t!(
        "crash.report_saved",
        path = report.path.display().to_string(),
        locale = &locale
    )
    .to_string()
    .into();
    let muted = cx.theme().muted_foreground;
    let mono = get_mono_font_family();
    Dialog::new(i18n_crash(cx, "title"))
        .icon(IconName::TriangleAlert)
        .child(move || {
            v_flex()
                .gap_2()
                .child(body.clone())
                .when(!summary.is_empty(), |this| {
                    this.child(div().font_family(mono.clone()).text_sm().child(summary.clone()))
                })
                .child(div().text_xs().text_color(muted).child(saved.clone()))
        })
        .ok_text(i18n_crash(cx, "open_logs"))
        .cancel_text(i18n_crash(cx, "dismiss"))
        .on_ok(|_, _window, cx| {
            match logs_dir() {
                Some(logs) => cx.open_with_system(&logs),
                None => error!("failed to resolve logs directory"),
            }
            true
        })
        .open(window, cx);
}

pub(crate) fn open_welcome_dialog(window: &mut Window, cx: &mut App) {
    let intro = i18n_hints(cx, "welcome_intro");
    let steps: [SharedString; 3] = [
        i18n_hints(cx, "welcome_step_home"),
        i18n_hints(cx, "welcome_step_todos"),
        format!(
            "{} ({})",
            i18n_hints(cx, "welcome_step_palette"),
            humanize_keystroke("secondary-k")
        )
        .into(),
    ];
    Dialog::new(i18n_hints(cx, "welcome_title"))
        .icon(IconName::Info)
        .child(move || v_flex().gap_2().child(intro.clone()).children(steps.iter().cloned()))
        .ok_text(i18n_hints(cx, "welcome_ok"))
        .open(window, cx);
}

pub(crate) fn open_install_quit_dialog(window: &mut Window, cx: &mut App) {
    Dialog::new(i18n_update(cx, "quit_to_install_title"))
        .icon(IconName::Info)
        .message(i18n_update(cx, "quit_to_install_body"))
        .ok_text(i18n_update(cx, "quit_to_install_now"))
        .cancel_text(i18n_update(cx, "quit_to_install_later"))
        .on_ok(|_, _window, cx| {
            focus_installer_ui();
            info!("update: quitting so the installer can replace the app");
            cx.quit();
            true
        })
        .open(window, cx);
}

pub(crate) fn open_update_dialog(info: UpdateInfo, root: WeakEntity<AppRoot>, window: &mut Window, cx: &mut App) {
    const MAX_NOTES: usize = 5000;
    let title = format!("{} {}", i18n_update(cx, "available_title"), info.version);
    let mut notes = info.notes.clone();
    if notes.chars().count() > MAX_NOTES {
        notes = notes.chars().take(MAX_NOTES).collect::<String>();
        notes.push('…');
    }
    let update_hint = i18n_update(cx, "update_body");
    let version_line = format!("{} → {}", info.current, info.version);
    let skip_version = info.version.clone();
    let download_info = info.clone();
    let downloaded = Rc::new(Cell::new(false));
    let on_download_flag = downloaded.clone();

    let on_download: DialogCallback = Rc::new(move |_window, cx| {
        on_download_flag.set(true);
        if let Some(view) = root.upgrade() {
            view.update(cx, |this, cx| this.start_download(download_info.clone(), cx));
        }
    });
    let skip = skip_version.clone();
    let on_skip: DialogCallback = Rc::new(move |_window, cx| {
        info!(version = %skip, "update: version skipped by user");
        let version = skip.clone();
        update_app_state_and_save_quiet(cx, "skip_update_version", move |state, _| {
            state.set_skipped_update_version(Some(version.clone()));
        });
        cx.global::<GlobalStore>().clone().update(cx, |state, cx| {
            state.set_available_update(None, cx);
        });
    });

    let actions = cx.new(|cx| UpdateDialog::new(on_download.clone(), on_skip.clone(), cx));
    Dialog::new(title)
        .child(move || {
            let mut body = v_flex()
                .gap_2()
                .child(Label::new(update_hint.clone()))
                .child(Label::new(version_line.clone()));
            if !notes.trim().is_empty() {
                let text = TextView::markdown("update-release-notes", notes.clone()).style(release_notes_style());
                let long_notes = notes.lines().count() > 12 || notes.chars().count() > 800;
                body = body.child(if long_notes {
                    div()
                        .w_full()
                        .h(px(280.))
                        .child(text)
                        .overflow_y_scrollbar()
                        .into_any_element()
                } else {
                    div().w_full().child(text).into_any_element()
                });
            }
            body
        })
        .footer_child(move || actions.clone().into_any_element())
        .w(px(520.))
        .overlay_closable(false)
        .on_close(move |_, _window, cx| {
            if !downloaded.get() {
                let version = skip_version.clone();
                update_app_state_and_save_quiet(cx, "skip_update_version", move |state, _| {
                    state.set_skipped_update_version(Some(version.clone()));
                });
                cx.global::<GlobalStore>().clone().update(cx, |state, cx| {
                    state.set_available_update(None, cx);
                });
            }
        })
        .open(window, cx);
}
