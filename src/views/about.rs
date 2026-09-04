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

use crate::assets::Assets;
use crate::constants::APP_NAME;
use crate::startup::{GIT_SHA, VERSION};
use crate::states::i18n_about;
use crate::views::secondary_window::{active_window_display, open_secondary_window};
use chrono::{Datelike, Local};
use gpui::{
    App, Bounds, Image, ImageFormat, TitlebarOptions, WindowBounds, WindowKind, WindowOptions, prelude::*, px, size,
};
use gpui_starter_ui::{AboutConfig, AboutLine, AboutLink, AboutPage};
use std::sync::Arc;

pub fn open_about_window(cx: &mut App) {
    let bounds = Bounds::centered(active_window_display(cx), size(px(420.), px(480.)), cx);
    let logo = Assets::get("icon.png").map(|item| item.data).unwrap_or_default();
    let logo = Arc::new(Image::from_bytes(ImageFormat::Png, logo.to_vec()));
    let year = Local::now().year();
    let repo = env!("CARGO_PKG_REPOSITORY");
    open_secondary_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_min_size: Some(size(px(360.), px(400.))),
            kind: WindowKind::PopUp,
            titlebar: Some(TitlebarOptions {
                title: Some(i18n_about(cx, "title")),
                ..Default::default()
            }),
            ..Default::default()
        },
        cx,
        |_window, cx| {
            cx.new(|_| {
                AboutPage::new(AboutConfig {
                    name: APP_NAME.into(),
                    logo,
                    lines: vec![
                        AboutLine::sm(format!("v{VERSION} ({GIT_SHA})")),
                        AboutLine::xs(format!("© {year} Andy Hsu")),
                    ],
                    links: vec![AboutLink::new("github", "GitHub", repo)],
                    system_info_collector: None,
                })
            })
        },
    );
}
