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

use crate::helpers::{MemuAction, SettingsAction};
use crate::states::i18n_tray;
use gpui::App;
use tracing::error;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIconBuilder};

const MENU_ID_QUIT: &str = "quit";
const MENU_ID_SHOW: &str = "show";
const MENU_ID_PREFERENCES: &str = "preferences";

fn load_icon() -> Icon {
    let icon_bytes = include_bytes!("../assets/icon.png");
    let img = image::load_from_memory(icon_bytes).expect("Failed to load tray icon");
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Icon::from_rgba(rgba.into_raw(), width, height).expect("Failed to create tray icon")
}

pub fn init_tray(cx: &mut App) {
    let menu = Menu::new();
    let _ = menu.append(&MenuItem::with_id(MENU_ID_SHOW, i18n_tray(cx, "show"), true, None));
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&MenuItem::with_id(
        MENU_ID_PREFERENCES,
        i18n_tray(cx, "preferences"),
        true,
        None,
    ));
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&MenuItem::with_id(MENU_ID_QUIT, i18n_tray(cx, "quit"), true, None));

    match TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip(crate::constants::APP_NAME)
        .with_icon(load_icon())
        .build()
    {
        Ok(_) => {}
        Err(e) => error!(error = %e, "failed to create tray icon"),
    }

    let rx = MenuEvent::receiver();
    cx.spawn(async move |cx| {
        loop {
            while let Ok(event) = rx.try_recv() {
                let id = event.id.0;
                cx.update(|cx| match id.as_str() {
                    MENU_ID_QUIT => cx.dispatch_action(&MemuAction::Quit),
                    MENU_ID_SHOW => {
                        cx.activate(true);
                        for handle in cx.windows() {
                            let _ = handle.update(cx, |_, window, _| window.activate_window());
                        }
                    }
                    MENU_ID_PREFERENCES => cx.dispatch_action(&SettingsAction::Open),
                    _ => {}
                });
            }
            cx.background_executor()
                .timer(std::time::Duration::from_millis(200))
                .await;
        }
    })
    .detach();
}
