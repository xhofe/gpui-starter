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

//! Window placement and theme application at launch and on change.

use crate::helpers::apply_default_ui_font_size;
use crate::states::AppState;
use gpui::{App, Bounds, Pixels, WindowAppearance, px, size};
use gpui_kit::component::{Theme, ThemeMode, ThemeRegistry};
use std::rc::Rc;
use tracing::info;

pub(crate) fn default_window_bounds(cx: &mut App) -> Bounds<Pixels> {
    let mut window_size = size(px(1200.), px(750.));
    if let Some(display) = cx.primary_display() {
        let ds = display.bounds().size;
        window_size.width = window_size.width.min(ds.width * 0.85);
        window_size.height = window_size.height.min(ds.height * 0.85);
    }
    Bounds::centered(None, window_size, cx)
}

pub(crate) fn resolve_window_bounds(state: &AppState, cx: &mut App) -> (Bounds<Pixels>, bool) {
    let clamp_to = |mut b: Bounds<Pixels>, screen: Bounds<Pixels>| -> Bounds<Pixels> {
        b.size = b.size.min(&screen.size);
        let max_x = screen.origin.x + screen.size.width - b.size.width;
        let max_y = screen.origin.y + screen.size.height - b.size.height;
        b.origin.x = b.origin.x.clamp(screen.origin.x, max_x);
        b.origin.y = b.origin.y.clamp(screen.origin.y, max_y);
        b
    };

    let displays: Vec<(String, Bounds<Pixels>)> = cx
        .displays()
        .into_iter()
        .filter_map(|d| Some((d.uuid().ok()?.to_string(), d.bounds())))
        .collect();
    let primary_uuid = cx.primary_display().and_then(|d| d.uuid().ok()).map(|u| u.to_string());

    let placement = state
        .window_placements()
        .iter()
        .find(|p| primary_uuid.as_deref() == Some(p.display_uuid.as_str()))
        .or_else(|| {
            state
                .window_placements()
                .iter()
                .find(|p| displays.iter().any(|(uuid, _)| uuid == &p.display_uuid))
        });
    if let Some(p) = placement
        && let Some((_, screen)) = displays.iter().find(|(uuid, _)| uuid == &p.display_uuid)
    {
        return (clamp_to(p.bounds + screen.origin, *screen), p.maximized);
    }

    if let Some(&saved) = state.bounds() {
        let area = |screen: &Bounds<Pixels>| {
            let i = saved.intersect(screen);
            if i.is_empty() {
                0.0
            } else {
                i.size.width.as_f32() * i.size.height.as_f32()
            }
        };
        if let Some((_, screen)) = displays
            .iter()
            .filter(|(_, b)| area(b) > 0.0)
            .max_by(|(_, a), (_, b)| area(a).total_cmp(&area(b)))
        {
            return (clamp_to(saved, *screen), false);
        }
    }

    info!("no usable saved window placement; centering on primary display");
    (default_window_bounds(cx), false)
}

pub(crate) fn apply_named_theme(name: &str, cx: &mut App) -> bool {
    let Some(config) = ThemeRegistry::global(cx).themes().get(name).cloned() else {
        return false;
    };
    Theme::global_mut(cx).apply_config(&config);
    apply_default_ui_font_size(cx);
    cx.refresh_windows();
    true
}

pub(crate) fn restore_default_themes(cx: &mut App) {
    let (mut light, mut dark) = {
        let registry = ThemeRegistry::global(cx);
        (
            (**registry.default_light_theme()).clone(),
            (**registry.default_dark_theme()).clone(),
        )
    };
    for cfg in [&mut light, &mut dark] {
        cfg.colors.primary = Some("#1f6feb".into());
        cfg.colors.primary_foreground = Some("#ffffff".into());
        cfg.colors.primary_hover = Some("#1a5ec8".into());
        cfg.colors.primary_active = Some("#1753b0".into());
    }
    let theme = Theme::global_mut(cx);
    theme.light_theme = Rc::new(light);
    theme.dark_theme = Rc::new(dark);
    apply_default_ui_font_size(cx);
}

pub(crate) fn theme_mode_for_appearance(appearance: WindowAppearance) -> ThemeMode {
    match appearance {
        WindowAppearance::Light | WindowAppearance::VibrantLight => ThemeMode::Light,
        _ => ThemeMode::Dark,
    }
}
