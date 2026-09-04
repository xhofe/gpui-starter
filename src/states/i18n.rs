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

use super::GlobalStore;
use gpui::{App, SharedString};
use rust_i18n::t;

pub fn i18n_common(cx: &App, key: &str) -> SharedString {
    let locale = cx.global::<GlobalStore>().read(cx).locale();
    t!(format!("common.{key}"), locale = locale).into()
}
pub fn i18n_sidebar(cx: &App, key: &str) -> SharedString {
    let locale = cx.global::<GlobalStore>().read(cx).locale();
    t!(format!("sidebar.{key}"), locale = locale).into()
}
pub fn i18n_settings(cx: &App, key: &str) -> SharedString {
    let locale = cx.global::<GlobalStore>().read(cx).locale();
    t!(format!("settings.{key}"), locale = locale).into()
}
pub fn i18n_home(cx: &App, key: &str) -> SharedString {
    let locale = cx.global::<GlobalStore>().read(cx).locale();
    t!(format!("home.{key}"), locale = locale).into()
}
pub fn i18n_todos(cx: &App, key: &str) -> SharedString {
    let locale = cx.global::<GlobalStore>().read(cx).locale();
    t!(format!("todos.{key}"), locale = locale).into()
}
pub fn i18n_about(cx: &App, key: &str) -> SharedString {
    let locale = cx.global::<GlobalStore>().read(cx).locale();
    t!(format!("about.{key}"), locale = locale).into()
}
pub fn i18n_update(cx: &App, key: &str) -> SharedString {
    let locale = cx.global::<GlobalStore>().read(cx).locale();
    t!(format!("update.{key}"), locale = locale).into()
}
pub fn i18n_crash(cx: &App, key: &str) -> SharedString {
    let locale = cx.global::<GlobalStore>().read(cx).locale();
    t!(format!("crash.{key}"), locale = locale).into()
}
pub fn i18n_hints(cx: &App, key: &str) -> SharedString {
    let locale = cx.global::<GlobalStore>().read(cx).locale();
    t!(format!("hints.{key}"), locale = locale).into()
}
pub fn i18n_shortcuts(cx: &App, key: &str) -> SharedString {
    let locale = cx.global::<GlobalStore>().read(cx).locale();
    t!(format!("shortcuts.{key}"), locale = locale).into()
}
pub fn i18n_command_palette(cx: &App, key: &str) -> SharedString {
    let locale = cx.global::<GlobalStore>().read(cx).locale();
    t!(format!("command_palette.{key}"), locale = locale).into()
}
#[cfg(not(target_os = "linux"))]
pub fn i18n_tray(cx: &App, key: &str) -> SharedString {
    let locale = cx.global::<GlobalStore>().read(cx).locale();
    t!(format!("tray.{key}"), locale = locale).into()
}
