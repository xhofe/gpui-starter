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

use crate::constants::{SIDEBAR_COLLAPSED_WIDTH, SIDEBAR_WIDTH};
use crate::error::Error;
use crate::helpers::{
    ConfigRecovery, TimeZonePref, UpdateInfo, get_or_create_config_dir, load_config_with_recovery, unix_ts,
    write_file_atomic_with_backup,
};
use gpui::{App, AppContext, Bounds, Context, Entity, EventEmitter, Global, Pixels, SharedString};
use gpui_kit::component::ThemeMode;
use gpui_kit::component::dialog::DialogButtonProps;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use sys_locale::get_locale;
use tracing::{error, info, warn};

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Route {
    #[default]
    Home,
    Todos,
    Settings,
}

impl Route {
    pub fn as_str(self) -> &'static str {
        match self {
            Route::Home => "home",
            Route::Todos => "todos",
            Route::Settings => "settings",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "home" | "" => Some(Route::Home),
            "todos" => Some(Route::Todos),
            "settings" => Some(Route::Settings),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Deserialize, JsonSchema, gpui::Action)]
pub enum ThemeAction {
    Light,
    Dark,
    System,
}

#[derive(Clone, PartialEq, Debug, Deserialize, JsonSchema, gpui::Action)]
pub struct SelectThemeAction {
    pub name: String,
}

#[derive(Clone, Copy, PartialEq, Debug, Deserialize, JsonSchema, gpui::Action)]
pub enum LocaleAction {
    En,
    Zh,
}

const LIGHT_THEME_MODE: &str = "light";
const DARK_THEME_MODE: &str = "dark";
const UPDATE_CHECK_INTERVAL: i64 = 2 * 24 * 60 * 60;
pub const HINT_WELCOME: &str = "welcome";
const MAX_WINDOW_PLACEMENTS: usize = 8;

#[derive(Clone, PartialEq, Debug, Deserialize, JsonSchema, Default)]
pub enum NotificationCategory {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Clone, PartialEq, Debug, Deserialize, JsonSchema, gpui::Action, Default)]
pub struct NotificationAction {
    pub title: Option<SharedString>,
    pub category: NotificationCategory,
    pub message: SharedString,
}

impl NotificationAction {
    pub fn new_info(message: SharedString) -> Self {
        Self {
            category: NotificationCategory::Info,
            message,
            ..Default::default()
        }
    }
    pub fn new_success(message: SharedString) -> Self {
        Self {
            category: NotificationCategory::Success,
            message,
            ..Default::default()
        }
    }
    pub fn new_warning(message: SharedString) -> Self {
        Self {
            category: NotificationCategory::Warning,
            message,
            ..Default::default()
        }
    }
    pub fn new_error(message: SharedString) -> Self {
        Self {
            category: NotificationCategory::Error,
            message,
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug)]
pub enum GlobalEvent {
    Notification(NotificationAction),
    RouteChanged,
    UpdateDownloadProgress,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowPlacement {
    pub display_uuid: String,
    pub bounds: Bounds<Pixels>,
    pub maximized: bool,
}

/// Persisted to `gpui-starter.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AppState {
    #[serde(skip)]
    route: Route,
    #[serde(rename = "route", default)]
    route_token: String,
    locale: Option<String>,
    bounds: Option<Bounds<Pixels>>,
    #[serde(default)]
    window_placements: Vec<WindowPlacement>,
    theme: Option<String>,
    theme_name: Option<String>,
    font_rem_px: Option<f32>,
    ui_font_family: Option<String>,
    mono_font_family: Option<String>,
    http_proxy: Option<String>,
    tray_enabled: Option<bool>,
    auto_update_check: Option<bool>,
    last_update_check: Option<i64>,
    skipped_update_version: Option<String>,
    include_prerelease: Option<bool>,
    time_zone: Option<String>,
    date_format: Option<String>,
    sidebar_collapsed: Option<bool>,
    dismissed_hints: Vec<String>,
    #[serde(default)]
    open_tabs: Vec<String>,
    #[serde(default)]
    active_tab: usize,
    #[serde(skip)]
    download_progress: Option<(u64, u64)>,
    #[serde(skip)]
    update_installed: bool,
    #[serde(skip)]
    update_checking: bool,
    #[serde(skip)]
    available_update: Option<UpdateInfo>,
}

impl EventEmitter<GlobalEvent> for AppState {}

#[derive(Debug, Clone)]
pub struct GlobalStore {
    app_state: Entity<AppState>,
}

impl GlobalStore {
    pub fn new(app_state: Entity<AppState>) -> Self {
        Self { app_state }
    }
    pub fn state(&self) -> Entity<AppState> {
        self.app_state.clone()
    }
    pub fn update<R, C: AppContext>(
        &self,
        cx: &mut C,
        update: impl FnOnce(&mut AppState, &mut Context<AppState>) -> R,
    ) -> R {
        self.app_state.update(cx, update)
    }
    pub fn read<'a>(&self, cx: &'a App) -> &'a AppState {
        self.app_state.read(cx)
    }
}

impl Global for GlobalStore {}

fn config_path() -> Result<PathBuf> {
    let path = get_or_create_config_dir()?.join("gpui-starter.toml");
    if !path.exists() {
        std::fs::write(&path, "")?;
    }
    Ok(path)
}

pub fn save_app_state(state: &AppState) -> Result<()> {
    let path = config_path()?;
    let value = toml::to_string(state)?;
    write_file_atomic_with_backup(&path, value.as_bytes())?;
    Ok(())
}

pub const SUPPORTED_LOCALES: [&str; 2] = ["en", "zh"];

pub fn language_from_system_locale(tag: Option<&str>) -> &'static str {
    let lang = tag
        .unwrap_or_default()
        .trim()
        .split(['-', '_', '.', '@'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    SUPPORTED_LOCALES
        .iter()
        .copied()
        .find(|bundled| *bundled == lang)
        .unwrap_or("en")
}

fn system_language() -> &'static str {
    static LANGUAGE: OnceLock<&'static str> = OnceLock::new();
    LANGUAGE.get_or_init(|| language_from_system_locale(get_locale().as_deref()))
}

impl AppState {
    pub fn new() -> Self {
        Self {
            locale: Some(system_language().to_string()),
            ..Default::default()
        }
    }

    pub fn try_new() -> Result<Self> {
        let path = config_path()?;
        let loaded = load_config_with_recovery(&path, |text| toml::from_str::<Self>(text).map_err(|e| e.to_string()))?;
        match &loaded.recovery {
            Some(ConfigRecovery::RestoredFromBackup { corrupt_path, .. }) => {
                warn!(corrupt = %corrupt_path.display(), "gpui-starter.toml was unreadable; restored from backup")
            }
            Some(ConfigRecovery::Reset { corrupt_path, .. }) => {
                error!(corrupt = %corrupt_path.display(), "gpui-starter.toml was unreadable and no backup parsed; reset to defaults")
            }
            None => {}
        }
        let mut state = loaded.value.unwrap_or_default();
        if state.locale.as_deref().is_none_or(|l| l.trim().is_empty()) {
            state.locale = Some(system_language().to_string());
        }
        state.route = Route::from_name(&state.route_token).unwrap_or(Route::Home);
        state.route_token = state.route.as_str().to_string();
        if state.open_tabs.is_empty() {
            state.open_tabs = vec![state.route.as_str().to_string()];
            state.active_tab = 0;
        }
        Ok(state)
    }

    pub fn go_to(&mut self, route: Route, cx: &mut Context<Self>) {
        self.route = route;
        self.route_token = route.as_str().to_string();
        if let Some(slot) = self.open_tabs.get_mut(self.active_tab) {
            *slot = route.as_str().to_string();
        }
        cx.emit(GlobalEvent::RouteChanged);
        cx.notify();
    }

    pub fn route(&self) -> Route {
        self.route
    }

    pub fn theme(&self) -> Option<ThemeMode> {
        match self.theme.as_deref() {
            Some(LIGHT_THEME_MODE) => Some(ThemeMode::Light),
            Some(DARK_THEME_MODE) => Some(ThemeMode::Dark),
            _ => None,
        }
    }

    pub fn set_theme(&mut self, mode: ThemeMode) {
        self.theme_name = None;
        self.theme = Some(
            match mode {
                ThemeMode::Light => LIGHT_THEME_MODE,
                ThemeMode::Dark => DARK_THEME_MODE,
            }
            .to_string(),
        );
    }

    pub fn set_theme_system(&mut self) {
        self.theme_name = None;
        self.theme = Some("system".to_string());
    }

    pub fn theme_name(&self) -> Option<String> {
        self.theme_name.clone()
    }

    pub fn set_theme_name(&mut self, name: String) {
        self.theme_name = Some(name);
    }

    pub fn locale(&self) -> &str {
        self.locale.as_deref().unwrap_or("en")
    }

    pub fn set_locale(&mut self, locale: String) {
        self.locale = Some(locale);
    }

    pub fn font_rem_px(&self) -> Option<f32> {
        self.font_rem_px
    }

    pub fn set_font_rem_px(&mut self, px: f32) {
        self.font_rem_px = Some(px);
    }

    pub fn ui_font_family(&self) -> Option<String> {
        self.ui_font_family.clone()
    }

    pub fn mono_font_family(&self) -> Option<String> {
        self.mono_font_family.clone()
    }

    pub fn http_proxy(&self) -> String {
        self.http_proxy.clone().unwrap_or_default()
    }

    pub fn set_http_proxy(&mut self, value: String) {
        self.http_proxy = if value.trim().is_empty() { None } else { Some(value) };
    }

    pub fn tray_enabled(&self) -> bool {
        self.tray_enabled.unwrap_or(false)
    }

    pub fn set_tray_enabled(&mut self, enabled: bool) {
        self.tray_enabled = Some(enabled);
    }

    pub fn auto_update_check(&self) -> bool {
        self.auto_update_check.unwrap_or(true)
    }

    pub fn set_auto_update_check(&mut self, enabled: bool) {
        self.auto_update_check = Some(enabled);
    }

    pub fn include_prerelease(&self) -> bool {
        self.include_prerelease.unwrap_or(false)
    }

    pub fn set_include_prerelease(&mut self, enabled: bool) {
        self.include_prerelease = Some(enabled);
    }

    pub fn update_check_due(&self) -> bool {
        match self.last_update_check {
            Some(ts) => unix_ts() - ts >= UPDATE_CHECK_INTERVAL,
            None => true,
        }
    }

    pub fn mark_update_checked(&mut self) {
        self.last_update_check = Some(unix_ts());
    }

    pub fn set_skipped_update_version(&mut self, version: Option<String>) {
        self.skipped_update_version = version;
    }

    pub fn time_zone(&self) -> TimeZonePref {
        TimeZonePref::from_name(self.time_zone.as_deref().unwrap_or("local"))
    }

    pub fn set_time_zone(&mut self, zone: TimeZonePref) {
        self.time_zone = Some(zone.name().to_string());
    }

    pub fn date_format(&self) -> String {
        self.date_format.clone().unwrap_or_else(|| "iso".to_string())
    }

    pub fn set_date_format(&mut self, id: String) {
        self.date_format = Some(id);
    }

    pub fn sidebar_collapsed(&self) -> bool {
        self.sidebar_collapsed.unwrap_or(false)
    }

    pub fn set_sidebar_collapsed(&mut self, collapsed: bool) {
        self.sidebar_collapsed = Some(collapsed);
    }

    pub fn sidebar_px(&self) -> Pixels {
        if self.sidebar_collapsed() {
            SIDEBAR_COLLAPSED_WIDTH
        } else {
            SIDEBAR_WIDTH
        }
    }

    pub fn bounds(&self) -> Option<&Bounds<Pixels>> {
        self.bounds.as_ref()
    }

    pub fn set_bounds(&mut self, bounds: Bounds<Pixels>) {
        self.bounds = Some(bounds);
    }

    pub fn window_placements(&self) -> &[WindowPlacement] {
        &self.window_placements
    }

    pub fn remember_placement(&mut self, placement: WindowPlacement) {
        self.window_placements
            .retain(|p| p.display_uuid != placement.display_uuid);
        self.window_placements.insert(0, placement);
        self.window_placements.truncate(MAX_WINDOW_PLACEMENTS);
    }

    pub fn hint_dismissed(&self, key: &str) -> bool {
        self.dismissed_hints.iter().any(|h| h == key)
    }

    pub fn dismiss_hint(&mut self, key: &str) {
        if !self.hint_dismissed(key) {
            self.dismissed_hints.push(key.to_string());
        }
    }

    pub fn open_tabs(&self) -> &[String] {
        &self.open_tabs
    }

    pub fn active_tab(&self) -> usize {
        self.active_tab.min(self.open_tabs.len().saturating_sub(1))
    }

    pub fn set_open_tabs(&mut self, tabs: Vec<String>, active: usize) {
        self.open_tabs = tabs;
        self.active_tab = active;
    }

    pub fn download_progress(&self) -> Option<(u64, u64)> {
        self.download_progress
    }

    pub fn set_download_progress(&mut self, progress: Option<(u64, u64)>, cx: &mut Context<Self>) {
        self.download_progress = progress;
        cx.emit(GlobalEvent::UpdateDownloadProgress);
        cx.notify();
    }

    pub fn update_installed(&self) -> bool {
        self.update_installed
    }

    pub fn set_update_installed(&mut self, installed: bool, cx: &mut Context<Self>) {
        self.update_installed = installed;
        cx.notify();
    }

    pub fn update_checking(&self) -> bool {
        self.update_checking
    }

    pub fn set_update_checking(&mut self, checking: bool, cx: &mut Context<Self>) {
        self.update_checking = checking;
        cx.notify();
    }

    pub fn available_update(&self) -> Option<UpdateInfo> {
        self.available_update.clone()
    }

    pub fn set_available_update(&mut self, info: Option<UpdateInfo>, cx: &mut Context<Self>) {
        self.available_update = info;
        cx.notify();
    }

    pub fn update_skipped(&self, version: &str) -> bool {
        self.skipped_update_version.as_deref() == Some(version)
    }

    pub fn redacted_toml(&self) -> String {
        let mut clone = self.clone();
        if clone.http_proxy.as_ref().is_some_and(|p| !p.is_empty()) {
            clone.http_proxy = Some("***".into());
        }
        toml::to_string(&clone).unwrap_or_default()
    }
}

const SAVE_DEBOUNCE: Duration = Duration::from_millis(500);
static SAVE_GENERATION: AtomicU64 = AtomicU64::new(0);

pub fn flush_app_state_on_quit(cx: &App) {
    cx.on_app_quit(|cx| {
        let state = cx.try_global::<GlobalStore>().map(|store| store.read(cx).clone());
        async move {
            let Some(state) = state else {
                return;
            };
            match save_app_state(&state) {
                Ok(()) => info!("flushed app state before quitting"),
                Err(e) => error!(error = %e, "failed to flush app state before quitting"),
            }
        }
    })
    .detach();
}

fn apply_and_save<F>(cx: &App, action_name: &'static str, refresh: bool, debounce: bool, mutation: F)
where
    F: FnOnce(&mut AppState, &App) + Send + 'static + Clone,
{
    let store = cx.global::<GlobalStore>().clone();
    cx.spawn(async move |cx| {
        store.update(cx, |state, cx| mutation(state, cx));
        if debounce {
            let generation = SAVE_GENERATION.fetch_add(1, Ordering::AcqRel) + 1;
            cx.background_executor().timer(SAVE_DEBOUNCE).await;
            if SAVE_GENERATION.load(Ordering::Acquire) != generation {
                return;
            }
        }
        let state = store.update(cx, |state, _| state.clone());
        cx.background_executor()
            .spawn(async move {
                if let Err(e) = save_app_state(&state) {
                    error!(error = %e, action = action_name, "Failed to save state");
                } else {
                    info!(action = action_name, "State saved successfully");
                }
            })
            .await;
        if refresh {
            cx.update(|cx| cx.refresh_windows());
        }
    })
    .detach();
}

pub fn update_app_state_and_save<F>(cx: &App, action_name: &'static str, mutation: F)
where
    F: FnOnce(&mut AppState, &App) + Send + 'static + Clone,
{
    apply_and_save(cx, action_name, true, false, mutation);
}

pub fn update_app_state_and_save_quiet<F>(cx: &App, action_name: &'static str, mutation: F)
where
    F: FnOnce(&mut AppState, &App) + Send + 'static + Clone,
{
    apply_and_save(cx, action_name, false, false, mutation);
}

pub fn update_app_state_and_save_debounced<F>(cx: &App, action_name: &'static str, mutation: F)
where
    F: FnOnce(&mut AppState, &App) + Send + 'static + Clone,
{
    apply_and_save(cx, action_name, true, true, mutation);
}

pub fn dialog_button_props(cx: &App) -> DialogButtonProps {
    DialogButtonProps::default()
        .cancel_text(super::i18n_common(cx, "cancel"))
        .ok_text(super::i18n_common(cx, "delete"))
}

pub fn notify(cx: &mut App, action: NotificationAction) {
    let store = cx.global::<GlobalStore>().state();
    store.update(cx, |state, cx| {
        cx.emit(GlobalEvent::Notification(action));
        let _ = state;
    });
}
