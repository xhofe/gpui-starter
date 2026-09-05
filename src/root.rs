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

//! Root view: sidebar, workspace tabs, title bar, and global actions.

use crate::constants::WORKSPACE_TAB_BAR_HEIGHT;
use crate::dialogs::*;
use crate::helpers::{
    ConfigRecovery, CrashReport, DEFAULT_UI_FONT_SIZE, Delivery, DiagnosticsAction, DiagnosticsInput, MemuAction,
    SettingsAction, UpdateInfo, WindowAction, WorkspaceTabAction, ZoomAction, download_and_verify, export_diagnostics,
    fetch_latest_release, get_or_create_config_dir, humanize_keystroke, install_update, installer_requires_quit,
    is_app_store_build,
};
use crate::startup::{GIT_SHA, VERSION};
use crate::states::{
    GlobalEvent, GlobalStore, LocaleAction, NotificationAction, NotificationCategory, Route, SelectThemeAction,
    ThemeAction, WindowPlacement, i18n_common, i18n_sidebar, i18n_update, notify, save_app_state,
    update_app_state_and_save, update_app_state_and_save_quiet,
};
use crate::views::{CommandPalette, Content, ShortcutsOverlay, Sidebar, TitleBar, open_settings_window};
use crate::window_setup::*;
use gpui::{
    Action, App, Bounds, Entity, MouseButton, Pixels, Point, SharedString, Subscription, Task, Window, div, prelude::*,
};
use gpui_kit::component::{
    ActiveTheme, IconName, Root, Sizable, Theme, ThemeMode, WindowExt,
    button::{Button, ButtonVariants},
    h_flex,
    label::Label,
    menu::ContextMenuExt,
    notification::Notification,
    v_flex,
};
use rust_i18n::t;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

const UI_ZOOM_MIN_PX: f32 = 12.0;
const UI_ZOOM_MAX_PX: f32 = 20.0;
pub(crate) const MAX_TABS: usize = 8;

struct ContentTab {
    route: Route,
    content: Entity<Content>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, Action)]
pub(crate) enum TabAction {
    Close(usize),
    CloseOthers(usize),
    CloseRight(usize),
}

pub(crate) struct DraggedTab {
    from: usize,
}

pub(crate) struct TabDragPreview {
    title: SharedString,
}

impl Render for TabDragPreview {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .rounded_md()
            .bg(cx.theme().background)
            .border_1()
            .border_color(cx.theme().border)
            .child(Label::new(self.title.clone()).text_sm())
    }
}

pub struct AppRoot {
    pending_notification: Option<Notification>,
    last_bounds: Bounds<Pixels>,
    save_task: Option<Task<()>>,
    sidebar: Entity<Sidebar>,
    tabs: Vec<ContentTab>,
    active_tab: usize,
    pending_new_tab: bool,
    command_palette: Entity<CommandPalette>,
    shortcuts_overlay: Entity<ShortcutsOverlay>,
    title_bar: Option<Entity<TitleBar>>,
    pub(crate) pending_update: Option<UpdateInfo>,
    update_task: Option<Task<()>>,
    download_task: Option<Task<()>>,
    pending_install_quit: bool,
    pub(crate) pending_welcome: bool,
    pub(crate) pending_config_recoveries: Vec<ConfigRecovery>,
    pub(crate) pending_crash: Option<CrashReport>,
    _global_sub: Subscription,
}

impl AppRoot {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let sidebar = cx.new(|cx| Sidebar::new(cx));
        let content = cx.new(|cx| Content::new(window, cx));
        let mut tabs = vec![ContentTab {
            route: Route::Home,
            content,
        }];
        let mut active_tab = 0;
        let (saved_tabs, saved_active) = {
            let store = cx.global::<GlobalStore>().read(cx);
            (
                store
                    .open_tabs()
                    .iter()
                    .filter_map(|name| Route::from_name(name))
                    .collect::<Vec<_>>(),
                store.active_tab(),
            )
        };
        if let Some(route) = saved_tabs.first() {
            tabs[0].route = *route;
        }
        for route in saved_tabs.iter().skip(1) {
            let content = cx.new(|cx| Content::new(window, cx));
            tabs.push(ContentTab { route: *route, content });
        }
        if saved_active < tabs.len() {
            active_tab = saved_active;
        }
        let command_palette = cx.new(|cx| CommandPalette::new(window, cx));
        let shortcuts_overlay = cx.new(ShortcutsOverlay::new);
        let title_bar = Some(cx.new(|cx| TitleBar::new(cx)));
        let global_state = cx.global::<GlobalStore>().state();
        let _global_sub = cx.subscribe(&global_state, |this, _state, event, cx| match event {
            GlobalEvent::Notification(e) => {
                let message = e.message.clone();
                let mut notification = match e.category {
                    NotificationCategory::Info => Notification::info(message),
                    NotificationCategory::Success => Notification::success(message),
                    NotificationCategory::Warning => Notification::warning(message),
                    NotificationCategory::Error => Notification::error(message),
                };
                if let Some(title) = e.title.as_ref().filter(|title| !title.is_empty()) {
                    notification = notification.title(title);
                }
                this.pending_notification = Some(notification);
                cx.notify();
            }
            GlobalEvent::RouteChanged => {
                let route = cx.global::<GlobalStore>().read(cx).route();
                this.tabs[this.active_tab].route = route;
                this.persist_tabs(cx);
                cx.notify();
            }
            GlobalEvent::UpdateDownloadProgress => cx.notify(),
        });

        if !tabs.is_empty() {
            let route = tabs[active_tab].route;
            cx.global::<GlobalStore>()
                .clone()
                .update(cx, |state, cx| state.go_to(route, cx));
        }

        Self {
            pending_notification: None,
            last_bounds: Bounds::default(),
            save_task: None,
            sidebar,
            tabs,
            active_tab,
            pending_new_tab: false,
            command_palette,
            shortcuts_overlay,
            title_bar,
            pending_update: None,
            update_task: None,
            download_task: None,
            pending_install_quit: false,
            pending_welcome: false,
            pending_config_recoveries: Vec::new(),
            pending_crash: None,
            _global_sub,
        }
    }

    fn persist_tabs(&self, cx: &mut Context<Self>) {
        let tabs: Vec<String> = self.tabs.iter().map(|tab| tab.route.as_str().to_string()).collect();
        let active = self.active_tab;
        update_app_state_and_save_quiet(cx, "save_open_tabs", move |state, _| {
            state.set_open_tabs(tabs.clone(), active)
        });
    }

    fn activate_tab(&mut self, ix: usize, cx: &mut Context<Self>) {
        if ix == self.active_tab || ix >= self.tabs.len() {
            return;
        }
        self.tabs[self.active_tab].route = cx.global::<GlobalStore>().read(cx).route();
        self.active_tab = ix;
        let route = self.tabs[ix].route;
        cx.global::<GlobalStore>()
            .clone()
            .update(cx, |state, cx| state.go_to(route, cx));
        self.persist_tabs(cx);
        cx.notify();
    }

    fn new_tab(&mut self, cx: &mut Context<Self>) {
        if self.tabs.len() >= MAX_TABS {
            return;
        }
        self.pending_new_tab = true;
        cx.notify();
    }

    fn close_tab(&mut self, ix: usize, cx: &mut Context<Self>) {
        if self.tabs.len() <= 1 || ix >= self.tabs.len() {
            return;
        }
        let was_active = ix == self.active_tab;
        self.tabs.remove(ix);
        if self.active_tab > ix {
            self.active_tab -= 1;
        } else if was_active {
            self.active_tab = ix.min(self.tabs.len() - 1);
            let route = self.tabs[self.active_tab].route;
            cx.global::<GlobalStore>()
                .clone()
                .update(cx, |state, cx| state.go_to(route, cx));
        }
        self.persist_tabs(cx);
        cx.notify();
    }

    fn close_others(&mut self, ix: usize, cx: &mut Context<Self>) {
        if ix >= self.tabs.len() || self.tabs.len() <= 1 {
            return;
        }
        let keep = self.tabs.remove(ix);
        self.tabs.clear();
        self.tabs.push(keep);
        self.active_tab = 0;
        let route = self.tabs[0].route;
        cx.global::<GlobalStore>()
            .clone()
            .update(cx, |state, cx| state.go_to(route, cx));
        self.persist_tabs(cx);
        cx.notify();
    }

    fn close_right(&mut self, ix: usize, cx: &mut Context<Self>) {
        if ix + 1 >= self.tabs.len() {
            return;
        }
        let active_closed = self.active_tab > ix;
        self.tabs.truncate(ix + 1);
        if active_closed {
            self.active_tab = ix;
            let route = self.tabs[ix].route;
            cx.global::<GlobalStore>()
                .clone()
                .update(cx, |state, cx| state.go_to(route, cx));
        }
        self.persist_tabs(cx);
        cx.notify();
    }

    fn move_tab(&mut self, from: usize, to: usize, cx: &mut Context<Self>) {
        if from == to || from >= self.tabs.len() || to >= self.tabs.len() {
            return;
        }
        let tab = self.tabs.remove(from);
        self.tabs.insert(to, tab);
        self.active_tab = moved_active_index(self.active_tab, from, to);
        self.persist_tabs(cx);
        cx.notify();
    }

    fn export_diagnostics(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let info = os_info::get();
        let store = cx.global::<GlobalStore>().read(cx);
        let summary = format!(
            "GPUI Starter diagnostics\nversion: {VERSION} ({GIT_SHA})\nos: {}-{}\narch: {}\nconfig_dir: {}\nlocale: {}\ntheme: {:?} / {:?}\napp_store_build: {}\ntime: {}\n",
            info.os_type(),
            info.version(),
            info.architecture().unwrap_or_default(),
            get_or_create_config_dir()
                .map(|d| d.display().to_string())
                .unwrap_or_default(),
            store.locale(),
            store.theme(),
            store.theme_name(),
            is_app_store_build(),
            chrono::Local::now().to_rfc3339(),
        );
        let app_config = store.redacted_toml();
        let locale = store.locale().to_string();
        let input = DiagnosticsInput { summary, app_config };
        match export_diagnostics(&input) {
            Ok(path) => {
                info!(path = %path.display(), "diagnostics bundle written");
                let message = t!(
                    "sidebar.diagnostics_saved",
                    path = path.display().to_string(),
                    locale = &locale
                );
                notify(cx, NotificationAction::new_success(message.to_string().into()));
                cx.reveal_path(&path);
            }
            Err(e) => {
                error!(error = %e, "diagnostics bundle failed");
                let message = t!("sidebar.diagnostics_failed", error = e.to_string(), locale = &locale);
                notify(cx, NotificationAction::new_error(message.to_string().into()));
            }
        }
    }

    pub(crate) fn check_for_updates(&mut self, manual: bool, then_prompt: bool, cx: &mut Context<Self>) {
        if is_app_store_build() || self.update_task.is_some() {
            return;
        }
        update_app_state_and_save_quiet(cx, "mark_update_checked", |state, _| state.mark_update_checked());
        cx.global::<GlobalStore>()
            .clone()
            .update(cx, |state, cx| state.set_update_checking(true, cx));
        let include_prerelease = cx.global::<GlobalStore>().read(cx).include_prerelease();
        self.update_task = Some(cx.spawn(async move |handle, cx| {
            let result = cx
                .background_spawn(async move { fetch_latest_release(include_prerelease) })
                .await;
            let _ = handle.update(cx, |this, cx| {
                this.update_task = None;
                let mut opened_prompt = false;
                match result {
                    Ok(Some(info)) => {
                        let skipped = cx.global::<GlobalStore>().read(cx).update_skipped(&info.version);
                        if manual || !skipped {
                            let version = info.version.clone();
                            cx.global::<GlobalStore>().clone().update(cx, |state, cx| {
                                state.set_available_update(Some(info.clone()), cx);
                            });
                            if then_prompt {
                                this.pending_update = Some(info);
                                opened_prompt = true;
                            } else {
                                let found = i18n_update(cx, "found");
                                let hint = i18n_update(cx, "manual_hint");
                                notify(
                                    cx,
                                    NotificationAction::new_info(format!("{found}: v{version}\n{hint}").into()),
                                );
                            }
                        }
                    }
                    Ok(None) => {
                        cx.global::<GlobalStore>().clone().update(cx, |state, cx| {
                            state.set_available_update(None, cx);
                        });
                        if manual {
                            let message = i18n_update(cx, "up_to_date");
                            notify(cx, NotificationAction::new_success(message));
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "update check failed");
                        if manual {
                            let message = i18n_update(cx, "check_failed");
                            notify(cx, NotificationAction::new_error(message));
                        }
                    }
                }
                if !opened_prompt {
                    cx.global::<GlobalStore>()
                        .clone()
                        .update(cx, |state, cx| state.set_update_checking(false, cx));
                }
                cx.notify();
            });
        }));
    }

    pub(crate) fn start_download(&mut self, info: UpdateInfo, cx: &mut Context<Self>) {
        let Some(asset) = info.asset.clone() else {
            cx.open_url(&info.page_url);
            return;
        };
        if self.download_task.is_some() {
            return;
        }
        let version = info.version.clone();
        let page_url = info.page_url.clone();
        cx.global::<GlobalStore>().clone().update(cx, |state, cx| {
            state.set_download_progress(Some((0, asset.size)), cx);
            state.set_update_installed(false, cx);
        });
        cx.notify();

        let (tx, rx) = smol::channel::unbounded::<(u64, u64)>();
        cx.spawn(async move |_, cx| {
            while let Ok(progress) = rx.recv().await {
                cx.update(|cx| {
                    cx.global::<GlobalStore>().clone().update(cx, |state, cx| {
                        state.set_download_progress(Some(progress), cx);
                    });
                });
            }
            cx.update(|cx| {
                cx.global::<GlobalStore>().clone().update(cx, |state, cx| {
                    state.set_download_progress(None, cx);
                });
            });
        })
        .detach();

        let log_name = asset.name.clone();
        self.download_task = Some(cx.spawn(async move |handle, cx| {
            let result = cx
                .background_spawn(async move {
                    let mut last_pct = u8::MAX;
                    let outcome = download_and_verify(&asset, |done, total| {
                        if total == 0 {
                            return;
                        }
                        let pct = ((done * 100 / total).min(100)) as u8;
                        if pct == last_pct {
                            return;
                        }
                        last_pct = pct;
                        let _ = tx.try_send((done, total));
                    })
                    .and_then(|path| install_update(&path));
                    drop(tx);
                    let _ = log_name;
                    outcome
                })
                .await;
            let _ = handle.update(cx, |this, cx| {
                this.download_task = None;
                match result {
                    #[cfg(target_os = "macos")]
                    Ok(Delivery::Replaced) => {
                        info!(version = %version, "update: installed in place, restart offered");
                        let message = i18n_update(cx, "installed_done");
                        notify(cx, NotificationAction::new_success(message));
                        cx.global::<GlobalStore>().clone().update(cx, |state, cx| {
                            state.set_update_installed(true, cx);
                        });
                    }
                    Ok(Delivery::HandedToOs) => {
                        info!(version = %version, "update: download finished, installer handed to the OS");
                        let message = i18n_update(cx, "download_done");
                        notify(cx, NotificationAction::new_success(message));
                        if installer_requires_quit() {
                            this.pending_install_quit = true;
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "update download failed");
                        let message = i18n_update(cx, "download_failed");
                        notify(cx, NotificationAction::new_warning(message));
                        cx.open_url(&page_url);
                    }
                }
                cx.notify();
            });
        }));
    }

    fn persist_window_state(
        &mut self,
        new_bounds: Bounds<Pixels>,
        display: Option<(String, Point<Pixels>)>,
        maximized: bool,
        cx: &mut Context<Self>,
    ) {
        self.last_bounds = new_bounds;
        let store = cx.global::<GlobalStore>().clone();
        let placement = display.map(|(display_uuid, screen_origin)| WindowPlacement {
            display_uuid,
            bounds: new_bounds - screen_origin,
            maximized,
        });
        let task = cx.spawn(async move |_, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(500))
                .await;
            let value = store.update(cx, move |state, cx| {
                if !maximized {
                    state.set_bounds(new_bounds);
                }
                if let Some(p) = placement {
                    state.remember_placement(p);
                }
                cx.notify();
                state.clone()
            });
            cx.background_spawn(async move {
                if let Err(e) = save_app_state(&value) {
                    error!(error = %e, "save window bounds fail");
                }
            })
            .await;
        });
        self.save_task = Some(task);
    }

    pub fn toggle_command_palette(&mut self, cx: &mut Context<Self>) {
        self.command_palette.update(cx, |palette, cx| palette.toggle(cx));
    }

    pub fn toggle_shortcuts(&mut self, cx: &mut Context<Self>) {
        self.shortcuts_overlay.update(cx, |overlay, cx| overlay.toggle(cx));
    }

    fn tab_title(&self, tab: &ContentTab, cx: &App) -> SharedString {
        match tab.route {
            Route::Home => i18n_sidebar(cx, "home"),
            Route::Todos => i18n_sidebar(cx, "todos"),
            Route::Settings => i18n_sidebar(cx, "preferences"),
        }
    }

    fn render_tab_bar(&mut self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        if self.tabs.len() <= 1 {
            return None;
        }
        let border = cx.theme().border;
        let foreground = cx.theme().foreground;
        let active_bg = foreground.alpha(0.1);
        let muted = cx.theme().muted_foreground;
        let titles: Vec<SharedString> = self.tabs.iter().map(|tab| self.tab_title(tab, cx)).collect();
        let strip = h_flex()
            .w_full()
            .h(WORKSPACE_TAB_BAR_HEIGHT)
            .flex_none()
            .gap_1()
            .px_2()
            .items_center()
            .border_b_1()
            .border_color(border)
            .children(titles.into_iter().enumerate().map(|(ix, title)| {
                let is_active = ix == self.active_tab;
                let shortcut: SharedString = humanize_keystroke(&format!("cmd-{}", ix + 1)).into();
                let shortcut_color = if is_active { muted } else { muted.alpha(0.6) };
                let title_color = if is_active { foreground } else { muted };
                let preview_title = title.clone();
                div()
                    .id(("content-tab", ix))
                    .flex_none()
                    .on_drag(DraggedTab { from: ix }, move |_, _, _, cx| {
                        let title = preview_title.clone();
                        cx.new(|_| TabDragPreview { title })
                    })
                    .on_drop(cx.listener(move |this, dragged: &DraggedTab, _window, cx| {
                        this.move_tab(dragged.from, ix, cx);
                    }))
                    .on_mouse_down(
                        MouseButton::Middle,
                        cx.listener(move |this, _, _window, cx| this.close_tab(ix, cx)),
                    )
                    .on_click(cx.listener(move |this, _, _window, cx| this.activate_tab(ix, cx)))
                    .child(
                        h_flex()
                            .gap_1()
                            .pl_2()
                            .pr_1()
                            .py_0p5()
                            .rounded_md()
                            .cursor_pointer()
                            .when(is_active, |this| this.bg(active_bg))
                            .when(!is_active, |this| this.text_color(muted))
                            .child(Label::new(title).text_sm().text_color(title_color).whitespace_nowrap())
                            .child(
                                Label::new(shortcut)
                                    .text_xs()
                                    .text_color(shortcut_color)
                                    .whitespace_nowrap(),
                            )
                            .child(
                                Button::new(("content-tab-close", ix))
                                    .ghost()
                                    .xsmall()
                                    .icon(IconName::Close)
                                    .on_click(cx.listener(move |this, _, _window, cx| {
                                        cx.stop_propagation();
                                        this.close_tab(ix, cx);
                                    })),
                            ),
                    )
                    .context_menu(move |menu, _window, cx| {
                        menu.menu(i18n_common(cx, "tab_close"), Box::new(TabAction::Close(ix)))
                            .menu(
                                i18n_common(cx, "tab_close_others"),
                                Box::new(TabAction::CloseOthers(ix)),
                            )
                            .menu(i18n_common(cx, "tab_close_right"), Box::new(TabAction::CloseRight(ix)))
                    })
            }));
        Some(strip)
    }
}

fn moved_active_index(active: usize, from: usize, to: usize) -> usize {
    if active == from {
        to
    } else if from < active && to >= active {
        active - 1
    } else if from > active && to <= active {
        active + 1
    } else {
        active
    }
}

impl Render for AppRoot {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let _ = Root::render_dialog_layer(window, cx);
        let _ = Root::render_notification_layer(window, cx);
        let current_bounds = window.bounds();
        if current_bounds != self.last_bounds {
            let display = window
                .display(cx)
                .and_then(|d| Some((d.uuid().ok()?.to_string(), d.bounds().origin)));
            self.persist_window_state(current_bounds, display, window.is_maximized(), cx);
        }
        if let Some(notification) = self.pending_notification.take() {
            window.push_notification(notification, cx);
        }
        for recovery in std::mem::take(&mut self.pending_config_recoveries) {
            let message = config_recovery_message(&recovery, cx);
            let notification = match recovery {
                ConfigRecovery::Reset { .. } => Notification::error(message),
                ConfigRecovery::RestoredFromBackup { .. } => Notification::warning(message),
            };
            window.push_notification(notification, cx);
        }
        if let Some(report) = self.pending_crash.take() {
            window.defer(cx, move |window, cx| open_crash_dialog(&report, window, cx));
        }
        if std::mem::take(&mut self.pending_install_quit) {
            open_install_quit_dialog(window, cx);
        }
        if std::mem::take(&mut self.pending_welcome) {
            window.defer(cx, open_welcome_dialog);
        }
        if let Some(info) = self.pending_update.take() {
            let weak = cx.entity().downgrade();
            open_update_dialog(info, weak, window, cx);
            cx.global::<GlobalStore>()
                .clone()
                .update(cx, |state, cx| state.set_update_checking(false, cx));
        }
        if let Some(font_size) = cx.global::<GlobalStore>().read(cx).font_rem_px() {
            window.set_rem_size(font_size);
        }
        if std::mem::take(&mut self.pending_new_tab) {
            let content = cx.new(|cx| Content::new(window, cx));
            self.tabs.push(ContentTab {
                route: Route::Home,
                content,
            });
            self.active_tab = self.tabs.len() - 1;
            cx.global::<GlobalStore>()
                .clone()
                .update(cx, |state, cx| state.go_to(Route::Home, cx));
            self.persist_tabs(cx);
        }

        let tab_bar = self.render_tab_bar(cx);
        let active_content = self.tabs[self.active_tab].content.clone();

        v_flex()
            .size_full()
            .on_action(cx.listener(|this, _: &DiagnosticsAction, window, cx| {
                this.export_diagnostics(window, cx);
            }))
            .on_action(cx.listener(|this, e: &WorkspaceTabAction, _window, cx| match *e {
                WorkspaceTabAction::New => this.new_tab(cx),
                WorkspaceTabAction::Select(ix) => this.activate_tab(ix, cx),
            }))
            .on_action(cx.listener(|this, e: &TabAction, _window, cx| match *e {
                TabAction::Close(ix) => this.close_tab(ix, cx),
                TabAction::CloseOthers(ix) => this.close_others(ix, cx),
                TabAction::CloseRight(ix) => this.close_right(ix, cx),
            }))
            .on_action(cx.listener(|_this, _: &SettingsAction, _window, cx| {
                open_settings_window(cx);
            }))
            .on_action(cx.listener(|_this, e: &ThemeAction, window, cx| match e {
                ThemeAction::Light => {
                    restore_default_themes(cx);
                    Theme::change(ThemeMode::Light, None, cx);
                    update_app_state_and_save(cx, "save_theme", |state, _| state.set_theme(ThemeMode::Light));
                }
                ThemeAction::Dark => {
                    restore_default_themes(cx);
                    Theme::change(ThemeMode::Dark, None, cx);
                    update_app_state_and_save(cx, "save_theme", |state, _| state.set_theme(ThemeMode::Dark));
                }
                ThemeAction::System => {
                    restore_default_themes(cx);
                    Theme::change(theme_mode_for_appearance(window.appearance()), None, cx);
                    update_app_state_and_save(cx, "save_theme", |state, _| state.set_theme_system());
                }
            }))
            .on_action(cx.listener(|_this, e: &SelectThemeAction, _window, cx| {
                if apply_named_theme(&e.name, cx) {
                    let name = e.name.clone();
                    update_app_state_and_save(cx, "save_theme_name", move |state, _| {
                        state.set_theme_name(name.clone());
                    });
                }
            }))
            .on_action(cx.listener(|_this, e: &LocaleAction, _window, cx| {
                let locale = match e {
                    LocaleAction::En => "en",
                    LocaleAction::Zh => "zh",
                };
                update_app_state_and_save(cx, "save_locale", move |state, _| {
                    state.set_locale(locale.to_string());
                });
            }))
            .on_action(cx.listener(|_this, e: &ZoomAction, _window, cx| {
                let current = cx
                    .global::<GlobalStore>()
                    .read(cx)
                    .font_rem_px()
                    .unwrap_or(DEFAULT_UI_FONT_SIZE);
                let next = match e {
                    ZoomAction::In => (current + 0.5).min(UI_ZOOM_MAX_PX),
                    ZoomAction::Out => (current - 0.5).max(UI_ZOOM_MIN_PX),
                    ZoomAction::Reset => DEFAULT_UI_FONT_SIZE,
                };
                Theme::global_mut(cx).font_size = gpui::px(next);
                update_app_state_and_save(cx, "zoom", move |state, _| {
                    state.set_font_rem_px(next);
                });
            }))
            .on_action(cx.listener(|_this, e: &WindowAction, window, _cx| match e {
                WindowAction::Minimize => window.minimize_window(),
                WindowAction::Zoom => window.zoom_window(),
                WindowAction::ToggleFullscreen => window.toggle_fullscreen(),
            }))
            // ⌘W closes the active workspace tab while more than one is open.
            // Otherwise propagate so the app-level handler can hide / close.
            .on_action(cx.listener(|this, e: &MemuAction, _window, cx| {
                if matches!(e, MemuAction::Close) && this.tabs.len() > 1 {
                    this.close_tab(this.active_tab, cx);
                } else {
                    cx.propagate();
                }
            }))
            .child(
                self.title_bar
                    .as_ref()
                    .map(|bar| bar.clone().into_any_element())
                    .unwrap_or_else(|| h_flex().into_any_element()),
            )
            .child(
                h_flex().flex_1().min_h_0().w_full().child(self.sidebar.clone()).child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .h_full()
                        .when_some(tab_bar, |this, bar| this.child(bar))
                        .child(v_flex().flex_1().min_h_0().w_full().child(active_content)),
                ),
            )
            .child(self.command_palette.clone())
            .child(self.shortcuts_overlay.clone())
    }
}
