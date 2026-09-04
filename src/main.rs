#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::helpers::{
    CrashContext, DiagnosticsAction, InstanceMessage, InstanceRole, MemuAction, PaletteAction, ShortcutsAction,
    UpdateAction, WindowAction, apply_default_ui_font_size, apply_fonts, claim_instance, get_or_create_config_dir,
    init_logger, install_panic_hook, instance_messages, is_app_store_build, load_keybinding_overrides, logs_dir,
    new_hot_keys, post_instance_message, release_instance, set_configured_proxy, set_datetime_prefs,
    take_config_recoveries, take_instance_server, take_pending_crash, with_app_identity,
};
use crate::states::{AppState, GlobalStore, HINT_WELCOME, flush_app_state_on_quit, update_app_state_and_save_quiet};
use crate::views::open_about_window;
#[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
use gpui::TitlebarOptions;
use gpui::{App, Bounds, Menu, MenuItem, OsAction, WindowBounds, WindowOptions, prelude::*, px, size};
use gpui_kit::component::input::{Copy, Cut, Paste, Redo, SelectAll, Undo};
use gpui_kit::component::{Root, Theme};
use gpui_starter_db::{init_database, open_failure_kind};
use sys_locale::get_locale;
use tracing::{error, info};

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

rust_i18n::i18n!(
    "locales_stub",
    fallback = "en",
    backend = crate::i18n_loader::runtime_backend()
);

mod assets;
mod constants;
mod dialogs;
mod error;
mod helpers;
mod i18n_loader;
mod root;
mod startup;
mod states;
#[cfg(not(target_os = "linux"))]
mod tray;
mod views;
mod window_setup;
use crate::constants::APP_NAME;
use crate::root::*;
use crate::startup::*;
use crate::window_setup::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _log_guard = init_logger()?;
    let info = os_info::get();
    let os = format!("{}-{}", info.os_type(), info.version());
    let arch = info.architecture().unwrap_or_default().to_string();
    install_panic_hook(CrashContext {
        version: VERSION,
        git_sha: GIT_SHA,
        os: os.clone(),
        arch: arch.clone(),
    });
    let config_dir = if let Ok(dir) = get_or_create_config_dir() {
        dir.to_string_lossy().to_string()
    } else {
        "--".to_string()
    };
    info!(
        version = VERSION,
        git_sha = GIT_SHA,
        os,
        arch,
        config_dir,
        is_app_store_build = is_app_store_build(),
        sys_locale = ?get_locale(),
        "gpui-starter launch"
    );
    if is_smoke_test() {
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_secs(30));
            eprintln!("GPUI_STARTER_SMOKE_TIMEOUT: no frame painted within 30s");
            std::process::exit(2);
        });
    }
    let app = gpui_platform::application().with_assets(assets::Assets);
    app.on_open_urls(|_urls| post_instance_message(InstanceMessage::default()));
    let app_state = AppState::try_new().unwrap_or_else(|e| {
        error!(error = %e, "gpui-starter.toml could not be loaded; starting with defaults");
        AppState::new()
    });
    if claim_instance(&InstanceMessage::default()) == InstanceRole::Forwarded {
        return Ok(());
    }
    let db_path = match database_path() {
        Ok(path) => path,
        Err(e) => {
            error!(error = %e, "config dir unavailable; showing the recovery window");
            run_db_recovery(
                app,
                app_state,
                gpui_starter_db::DbOpenFailure::Inaccessible(e.to_string()),
            );
            return Ok(());
        }
    };
    if let Err(e) = init_database(&db_path) {
        let failure = open_failure_kind(&e);
        error!(error = %e, failure = ?failure, "init database failed; showing the recovery window");
        run_db_recovery(app, app_state, failure);
        return Ok(());
    }
    app.run(move |cx| {
        gpui_kit::component::init(cx);
        launch(cx, app_state);
    });
    Ok(())
}

fn run_db_recovery(app: gpui::Application, app_state: AppState, failure: gpui_starter_db::DbOpenFailure) {
    app.run(move |cx| {
        gpui_kit::component::init(cx);
        let mode = match app_state.theme() {
            Some(m) => m,
            None => theme_mode_for_appearance(cx.window_appearance()),
        };
        Theme::change(mode, None, cx);
        apply_default_ui_font_size(cx);
        cx.activate(true);
        let bounds = Bounds::centered(None, size(px(540.), px(300.)), cx);
        let opened = cx.open_window(
            with_app_identity(WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(440.), px(240.))),
                ..Default::default()
            }),
            |window, cx| {
                window.on_window_should_close(cx, |_window, cx| {
                    cx.quit();
                    true
                });
                let view = cx.new(|_| DatabaseErrorView::new(failure, app_state));
                cx.new(|cx| Root::new(view, window, cx))
            },
        );
        if opened.is_err() {
            cx.quit();
        }
    });
}

fn activate_from_instance(_message: InstanceMessage, cx: &mut App) {
    cx.activate(true);
    for handle in cx.windows() {
        let _ = handle.update(cx, |_, window, _| window.activate_window());
    }
}

pub(crate) fn launch(cx: &mut App, app_state: AppState) {
    let fonts = ["fonts/JetBrainsMono-Regular.ttf", "fonts/JetBrainsMono-Bold.ttf"]
        .into_iter()
        .filter_map(|p| assets::Assets::get(p).map(|f| f.data))
        .collect();
    if let Err(e) = cx.text_system().add_fonts(fonts) {
        error!(error = %e, "failed to register bundled fonts");
    }
    assets::register_themes(cx);

    cx.activate(true);
    let (window_bounds, maximized) = resolve_window_bounds(&app_state, cx);
    info!(bounds = ?window_bounds, maximized, "resolved window bounds");
    let app_state = cx.new(|_| app_state);
    let app_store = GlobalStore::new(app_state);
    let saved_theme_name = app_store.read(cx).theme_name();
    let saved_mode = app_store.read(cx).theme();
    let applied = match saved_theme_name {
        Some(name) => apply_named_theme(&name, cx),
        None => false,
    };
    if !applied {
        let mode = match saved_mode {
            Some(m) => m,
            None => theme_mode_for_appearance(cx.window_appearance()),
        };
        Theme::change(mode, None, cx);
    }
    apply_default_ui_font_size(cx);
    if let Some(px) = app_store.read(cx).font_rem_px() {
        Theme::global_mut(cx).font_size = gpui::px(px);
    }
    cx.set_global(app_store);
    flush_app_state_on_quit(cx);
    {
        let (ui_font, mono_font) = {
            let store = cx.global::<GlobalStore>().read(cx);
            (store.ui_font_family(), store.mono_font_family())
        };
        apply_fonts(cx, ui_font.as_deref(), mono_font.as_deref());
    }
    {
        let proxy = cx.global::<GlobalStore>().read(cx).http_proxy();
        set_configured_proxy(&proxy);
        let store = cx.global::<GlobalStore>().read(cx);
        set_datetime_prefs(store.time_zone(), &store.date_format());
    }
    #[cfg(not(target_os = "linux"))]
    {
        let tray_enabled = cx.global::<GlobalStore>().read(cx).tray_enabled();
        if tray_enabled {
            tray::init_tray(cx);
        }
    }
    let overridden = load_keybinding_overrides();
    if overridden > 0 {
        info!(overridden, "keybinding overrides loaded");
    }
    cx.bind_keys(new_hot_keys());
    cx.on_action(|e: &MemuAction, cx: &mut App| match e {
        MemuAction::Quit => cx.quit(),
        MemuAction::About => open_about_window(cx),
        MemuAction::Close => {
            #[cfg(target_os = "macos")]
            cx.hide();
            #[cfg(not(target_os = "macos"))]
            if let Some(window) = cx.active_window() {
                let _ = window.update(cx, |_, window, _cx| window.remove_window());
            }
        }
        MemuAction::OpenLogs => match logs_dir() {
            Some(logs) => cx.open_with_system(&logs),
            None => error!("failed to resolve logs directory"),
        },
    });
    let mut menu_items = vec![MenuItem::action(format!("About {APP_NAME}"), MemuAction::About)];
    if !is_app_store_build() {
        menu_items.push(MenuItem::action("Check for Updates", UpdateAction::Check));
    }
    menu_items.extend([
        MenuItem::action("Open Logs Folder", MemuAction::OpenLogs),
        MenuItem::action("Export Diagnostics…", DiagnosticsAction::Export),
        MenuItem::action("Close Window", MemuAction::Close),
        MenuItem::action("Quit", MemuAction::Quit),
    ]);
    cx.set_menus(vec![
        Menu {
            name: APP_NAME.into(),
            items: menu_items,
            disabled: false,
        },
        Menu {
            name: "Edit".into(),
            items: vec![
                MenuItem::os_action("Undo", Undo, OsAction::Undo),
                MenuItem::os_action("Redo", Redo, OsAction::Redo),
                MenuItem::separator(),
                MenuItem::os_action("Cut", Cut, OsAction::Cut),
                MenuItem::os_action("Copy", Copy, OsAction::Copy),
                MenuItem::os_action("Paste", Paste, OsAction::Paste),
                MenuItem::separator(),
                MenuItem::os_action("Select All", SelectAll, OsAction::SelectAll),
            ],
            disabled: false,
        },
        Menu {
            name: "Window".into(),
            items: vec![
                MenuItem::action("Minimize", WindowAction::Minimize),
                MenuItem::action("Zoom", WindowAction::Zoom),
                MenuItem::action("Toggle Full Screen", WindowAction::ToggleFullscreen),
                MenuItem::separator(),
                MenuItem::action("Close Window", MemuAction::Close),
            ],
            disabled: false,
        },
    ]);

    if let Some(server) = take_instance_server() {
        server.serve(post_instance_message);
    }
    let inbox = instance_messages();
    cx.spawn(async move |cx| {
        while let Ok(message) = inbox.recv().await {
            cx.update(|cx| activate_from_instance(message, cx));
        }
    })
    .detach();
    cx.on_app_quit(|_cx| async { release_instance() }).detach();

    cx.spawn(async move |cx| {
        cx.open_window(
            with_app_identity(WindowOptions {
                window_bounds: Some(if maximized {
                    WindowBounds::Maximized(window_bounds)
                } else {
                    WindowBounds::Windowed(window_bounds)
                }),
                #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
                titlebar: Some(TitlebarOptions {
                    title: None,
                    appears_transparent: true,
                    traffic_light_position: Some(gpui::point(px(9.0), px(9.0))),
                }),
                show: cfg!(not(target_os = "macos")),
                window_min_size: Some(size(px(600.), px(400.))),
                ..Default::default()
            }),
            |window, cx| {
                #[cfg(target_os = "macos")]
                window.on_window_should_close(cx, move |_window, cx| {
                    cx.hide();
                    false
                });
                #[cfg(target_os = "macos")]
                window.on_next_frame(|window, _cx| window.activate_window());
                if is_smoke_test() {
                    println!("GPUI_STARTER_SMOKE_WINDOW");
                    if smoke_gate_is_window() {
                        std::thread::spawn(|| {
                            std::thread::sleep(std::time::Duration::from_secs(5));
                            println!("GPUI_STARTER_SMOKE_OK (window gate)");
                            std::process::exit(0);
                        });
                    }
                    window.on_next_frame(|_window, _cx| {
                        println!("GPUI_STARTER_SMOKE_OK");
                        std::process::exit(0);
                    });
                }
                let root_view = cx.new(|cx| AppRoot::new(window, cx));
                {
                    let weak = root_view.downgrade();
                    cx.on_action(move |_: &PaletteAction, cx: &mut App| {
                        if let Some(view) = weak.upgrade() {
                            view.update(cx, |root, cx| root.toggle_command_palette(cx));
                        }
                    });
                }
                {
                    let weak = root_view.downgrade();
                    cx.on_action(move |_: &ShortcutsAction, cx: &mut App| {
                        if let Some(view) = weak.upgrade() {
                            view.update(cx, |root, cx| root.toggle_shortcuts(cx));
                        }
                    });
                }
                {
                    let weak = root_view.downgrade();
                    cx.on_action(move |e: &UpdateAction, cx: &mut App| {
                        let Some(view) = weak.upgrade() else {
                            return;
                        };
                        match e {
                            UpdateAction::Check => {
                                view.update(cx, |root, cx| root.check_for_updates(true, false, cx));
                            }
                            UpdateAction::OpenPrompt => {
                                let cached = cx.global::<GlobalStore>().read(cx).available_update();
                                view.update(cx, |root, cx| match cached {
                                    Some(info) => {
                                        root.pending_update = Some(info);
                                        cx.notify();
                                    }
                                    None => root.check_for_updates(true, true, cx),
                                });
                            }
                        }
                    });
                }
                let auto_due = {
                    let store = cx.global::<GlobalStore>().read(cx);
                    store.auto_update_check() && store.update_check_due()
                };
                if auto_due {
                    root_view.update(cx, |root, cx| root.check_for_updates(false, false, cx));
                }
                root_view.update(cx, |root, _| {
                    root.pending_config_recoveries = take_config_recoveries();
                    root.pending_crash = take_pending_crash();
                });
                if !cx.global::<GlobalStore>().read(cx).hint_dismissed(HINT_WELCOME) {
                    update_app_state_and_save_quiet(cx, "dismiss_hint_welcome", |state, _| {
                        state.dismiss_hint(HINT_WELCOME)
                    });
                    root_view.update(cx, |root, _| root.pending_welcome = true);
                }
                cx.new(|cx| Root::new(root_view, window, cx))
            },
        )?;
        Ok::<_, anyhow::Error>(())
    })
    .detach();
}
