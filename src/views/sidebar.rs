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

use crate::states::{GlobalStore, Route, i18n_sidebar, update_app_state_and_save};
use crate::views::open_about_window;
use gpui::{App, Window, div, prelude::*};
use gpui_kit::component::{
    ActiveTheme, IconName,
    button::{Button, ButtonVariants},
    h_flex, v_flex,
};

pub struct Sidebar;

impl Sidebar {
    pub fn new(_cx: &mut App) -> Self {
        Self
    }
}

impl Render for Sidebar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let store = cx.global::<GlobalStore>().read(cx);
        let collapsed = store.sidebar_collapsed();
        let route = store.route();
        let width = store.sidebar_px();

        v_flex()
            .w(width)
            .h_full()
            .p_2()
            .gap_1()
            .border_r_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().sidebar)
            .child(
                Button::new("nav-home")
                    .ghost()
                    .icon(IconName::LayoutDashboard)
                    .when(!collapsed, |b| b.label(i18n_sidebar(cx, "home")))
                    .when(route == Route::Home, |b| b.primary())
                    .on_click(|_, _, cx| {
                        cx.global::<GlobalStore>()
                            .clone()
                            .update(cx, |state, cx| state.go_to(Route::Home, cx));
                    }),
            )
            .child(
                Button::new("nav-todos")
                    .ghost()
                    .icon(IconName::Check)
                    .when(!collapsed, |b| b.label(i18n_sidebar(cx, "todos")))
                    .when(route == Route::Todos, |b| b.primary())
                    .on_click(|_, _, cx| {
                        cx.global::<GlobalStore>()
                            .clone()
                            .update(cx, |state, cx| state.go_to(Route::Todos, cx));
                    }),
            )
            .child(div().flex_1())
            .child(
                Button::new("nav-settings")
                    .ghost()
                    .icon(IconName::Settings)
                    .when(!collapsed, |b| b.label(i18n_sidebar(cx, "preferences")))
                    .on_click(|_, _, cx| crate::views::open_settings_window(cx)),
            )
            .child(
                Button::new("nav-about")
                    .ghost()
                    .icon(IconName::Info)
                    .when(!collapsed, |b| b.label(i18n_sidebar(cx, "about")))
                    .on_click(|_, _, cx| open_about_window(cx)),
            )
            .child(
                h_flex().child(
                    Button::new("sidebar-collapse")
                        .ghost()
                        .icon(if collapsed {
                            IconName::PanelLeftOpen
                        } else {
                            IconName::PanelLeftClose
                        })
                        .on_click(move |_, _, cx| {
                            update_app_state_and_save(cx, "toggle_sidebar", move |state, _| {
                                state.set_sidebar_collapsed(!collapsed);
                            });
                        }),
                ),
            )
    }
}
