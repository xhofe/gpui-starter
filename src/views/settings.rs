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

use crate::helpers::{
    DATE_FORMATS, TimeZonePref, date_format_sample, ensure_keybindings_file, is_valid_proxy_setting,
    set_configured_proxy, set_datetime_prefs,
};
use crate::states::{
    GlobalStore, LocaleAction, ThemeAction, i18n_settings, update_app_state_and_save,
    update_app_state_and_save_debounced,
};
use crate::views::secondary_window::{active_window_display, open_secondary_window};
use crate::window_setup::{apply_named_theme, restore_default_themes};
use gpui::{
    App, Bounds, Entity, Subscription, TitlebarOptions, Window, WindowBounds, WindowOptions, div, prelude::*, px, size,
};
use gpui_kit::component::{
    ActiveTheme, Theme, ThemeMode, ThemeRegistry,
    button::Button,
    h_flex,
    input::{Input, InputEvent, InputState},
    label::Label,
    scroll::ScrollableElement,
    slider::{Slider, SliderEvent, SliderState, SliderValue},
    switch::Switch,
    v_flex,
};
use gpui_starter_ui::{Select, SelectEvent};

pub fn open_settings_window(cx: &mut App) {
    let bounds = Bounds::centered(active_window_display(cx), size(px(640.), px(720.)), cx);
    open_secondary_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_min_size: Some(size(px(480.), px(400.))),
            titlebar: Some(TitlebarOptions {
                title: Some(i18n_settings(cx, "title")),
                ..Default::default()
            }),
            ..Default::default()
        },
        cx,
        |window, cx| cx.new(|cx| Settings::new(window, cx)),
    );
}

struct Settings {
    locale: Entity<Select>,
    time_zone: Entity<Select>,
    date_format: Entity<Select>,
    proxy: Entity<InputState>,
    font_slider: Entity<SliderState>,
    _subs: Vec<Subscription>,
}

impl Settings {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (locale_idx, tz_idx, date_format_id, proxy_value, font_px) = {
            let store = cx.global::<GlobalStore>().read(cx);
            let locale_idx = if store.locale() == "zh" { 1 } else { 0 };
            let tz_idx = match store.time_zone() {
                TimeZonePref::Local => 0,
                TimeZonePref::Utc => 1,
            };
            (
                locale_idx,
                tz_idx,
                store.date_format(),
                store.http_proxy(),
                store.font_rem_px().unwrap_or(14.0),
            )
        };
        let locale = cx.new(|cx| Select::new(vec!["English".into(), "中文".into()], Some(locale_idx), window, cx));
        let tz_labels: Vec<String> = TimeZonePref::ALL.iter().map(|zone| zone.label().to_string()).collect();
        let time_zone = cx.new(|cx| Select::new(tz_labels, Some(tz_idx), window, cx));
        let formats: Vec<String> = DATE_FORMATS.iter().map(date_format_sample).collect();
        let fmt_idx = DATE_FORMATS.iter().position(|f| f.id == date_format_id).unwrap_or(0);
        let date_format = cx.new(|cx| Select::new(formats, Some(fmt_idx), window, cx));
        let proxy = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("http://127.0.0.1:7890")
                .default_value(proxy_value)
        });
        let font_slider = cx.new(|_cx| {
            SliderState::new()
                .min(12.0)
                .max(20.0)
                .step(0.5)
                .default_value(SliderValue::Single(font_px))
        });

        let mut subs = Vec::new();
        subs.push(cx.subscribe(&locale, |_, _, event: &SelectEvent, cx| {
            let SelectEvent::Change(i) = event;
            let locale = if *i == 1 { "zh" } else { "en" };
            update_app_state_and_save(cx, "save_locale", move |state, _| {
                state.set_locale(locale.to_string());
            });
            let _ = LocaleAction::En;
        }));
        subs.push(cx.subscribe(&time_zone, |_, _, event: &SelectEvent, cx| {
            let SelectEvent::Change(i) = event;
            let zone = TimeZonePref::ALL.get(*i).copied().unwrap_or_default();
            update_app_state_and_save(cx, "save_tz", move |state, _| {
                state.set_time_zone(zone);
                set_datetime_prefs(zone, &state.date_format());
            });
        }));
        subs.push(cx.subscribe(&date_format, |_, _, event: &SelectEvent, cx| {
            let SelectEvent::Change(i) = event;
            let id = DATE_FORMATS
                .get(*i)
                .map(|f| f.id.to_string())
                .unwrap_or_else(|| "iso".into());
            update_app_state_and_save(cx, "save_date_format", move |state, _| {
                state.set_date_format(id.clone());
                set_datetime_prefs(state.time_zone(), &id);
            });
        }));
        subs.push(cx.subscribe(&proxy, |_, input, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change | InputEvent::Blur) {
                let value = input.read(cx).value().to_string();
                if is_valid_proxy_setting(&value) || value.trim().is_empty() {
                    set_configured_proxy(&value);
                    update_app_state_and_save_quiet_proxy(cx, value);
                }
            }
        }));
        subs.push(cx.subscribe(&font_slider, |_, _, event: &SliderEvent, cx| {
            if let SliderEvent::Change(SliderValue::Single(v)) = event {
                let v = *v;
                Theme::global_mut(cx).font_size = px(v);
                update_app_state_and_save_debounced(cx, "save_font", move |state, _| {
                    state.set_font_rem_px(v);
                });
            }
        }));

        Self {
            locale,
            time_zone,
            date_format,
            proxy,
            font_slider,
            _subs: subs,
        }
    }
}

fn update_app_state_and_save_quiet_proxy(cx: &App, value: String) {
    crate::states::update_app_state_and_save_quiet(cx, "save_proxy", move |state, _| {
        state.set_http_proxy(value.clone());
    });
}

fn set_mode(cx: &mut App, mode: ThemeMode) {
    restore_default_themes(cx);
    Theme::change(mode, None, cx);
    update_app_state_and_save(cx, "save_theme", move |state, _| state.set_theme(mode));
    let _ = ThemeAction::System;
}

impl Render for Settings {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let store = cx.global::<GlobalStore>().read(cx);
        let tray = store.tray_enabled();
        let auto_update = store.auto_update_check();
        let prerelease = store.include_prerelease();
        let theme_names: Vec<String> = ThemeRegistry::global(cx)
            .themes()
            .keys()
            .map(|s| s.to_string())
            .collect();

        v_flex()
            .size_full()
            .p_6()
            .gap_4()
            .overflow_y_scrollbar()
            .child(Label::new(i18n_settings(cx, "section_appearance")).font_weight(gpui::FontWeight::BOLD))
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("theme-light")
                            .label(i18n_settings(cx, "theme_light"))
                            .on_click(|_, _, cx| {
                                set_mode(cx, ThemeMode::Light);
                            }),
                    )
                    .child(
                        Button::new("theme-dark")
                            .label(i18n_settings(cx, "theme_dark"))
                            .on_click(|_, _, cx| {
                                set_mode(cx, ThemeMode::Dark);
                            }),
                    )
                    .child(
                        Button::new("theme-system")
                            .label(i18n_settings(cx, "theme_system"))
                            .on_click(|_, window, cx| {
                                restore_default_themes(cx);
                                Theme::change(
                                    crate::window_setup::theme_mode_for_appearance(window.appearance()),
                                    None,
                                    cx,
                                );
                                update_app_state_and_save(cx, "save_theme", |state, _| state.set_theme_system());
                            }),
                    ),
            )
            .when(!theme_names.is_empty(), |this| {
                this.child(
                    Label::new(i18n_settings(cx, "named_themes"))
                        .text_sm()
                        .text_color(cx.theme().muted_foreground),
                )
                .child(
                    h_flex()
                        .gap_2()
                        .flex_wrap()
                        .children(theme_names.into_iter().map(|name| {
                            let label = name.clone();
                            Button::new(format!("theme-{name}"))
                                .outline()
                                .label(label)
                                .on_click(move |_, _, cx| {
                                    if apply_named_theme(&name, cx) {
                                        let n = name.clone();
                                        update_app_state_and_save(cx, "save_theme_name", move |state, _| {
                                            state.set_theme_name(n.clone());
                                        });
                                    }
                                })
                        })),
                )
            })
            .child(Label::new(i18n_settings(cx, "lang")))
            .child(self.locale.clone())
            .child(Label::new(i18n_settings(cx, "font_size")))
            .child(Slider::new(&self.font_slider))
            .child(Label::new(i18n_settings(cx, "section_datetime")).font_weight(gpui::FontWeight::BOLD))
            .child(Label::new(i18n_settings(cx, "time_zone")))
            .child(self.time_zone.clone())
            .child(Label::new(i18n_settings(cx, "date_format")))
            .child(self.date_format.clone())
            .child(Label::new(i18n_settings(cx, "section_system")).font_weight(gpui::FontWeight::BOLD))
            .child(Label::new(i18n_settings(cx, "http_proxy")))
            .child(div().h(px(32.)).child(Input::new(&self.proxy)))
            .child(
                h_flex()
                    .gap_2()
                    .child(Label::new(i18n_settings(cx, "tray_enabled")))
                    .child(Switch::new("tray").checked(tray).on_click(|checked, _, cx| {
                        let enabled = *checked;
                        update_app_state_and_save(cx, "save_tray", move |state, _| state.set_tray_enabled(enabled));
                    })),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(Label::new(i18n_settings(cx, "auto_update")))
                    .child(
                        Switch::new("auto-update")
                            .checked(auto_update)
                            .on_click(|checked, _, cx| {
                                let enabled = *checked;
                                update_app_state_and_save(cx, "save_auto_update", move |state, _| {
                                    state.set_auto_update_check(enabled);
                                });
                            }),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(Label::new(i18n_settings(cx, "prerelease")))
                    .child(
                        Switch::new("prerelease")
                            .checked(prerelease)
                            .on_click(|checked, _, cx| {
                                let enabled = *checked;
                                update_app_state_and_save(cx, "save_prerelease", move |state, _| {
                                    state.set_include_prerelease(enabled);
                                });
                            }),
                    ),
            )
            .child(
                Button::new("open-keybindings")
                    .outline()
                    .label(i18n_settings(cx, "keybindings_open"))
                    .on_click(|_, _, cx| {
                        if let Ok(path) = ensure_keybindings_file() {
                            cx.open_with_system(&path);
                        }
                    }),
            )
    }
}
