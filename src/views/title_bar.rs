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

use crate::constants::APP_NAME;
use crate::helpers::{DiagnosticsAction, UpdateAction, is_app_store_build};
use crate::states::{GlobalStore, i18n_sidebar, i18n_update};
use crate::views::open_about_window;
use gpui::{App, Window, div, prelude::*};
use gpui_kit::component::{
    IconName, TitleBar as KitTitleBar,
    button::{Button, ButtonVariants},
    h_flex,
    label::Label,
};

pub struct TitleBar;

impl TitleBar {
    pub fn new(_cx: &mut App) -> Self {
        Self
    }
}

impl Render for TitleBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let checking = cx.global::<GlobalStore>().read(cx).update_checking();
        KitTitleBar::new().child(
            h_flex()
                .w_full()
                .items_center()
                .px_2()
                .gap_2()
                .child(Label::new(APP_NAME).text_sm())
                .child(div().flex_1())
                .when(!is_app_store_build(), |this| {
                    this.child(
                        Button::new("title-update")
                            .ghost()
                            .icon(IconName::ArrowUp)
                            .tooltip(i18n_update(cx, "check"))
                            .loading(checking)
                            .on_click(|_, window, cx| {
                                window.dispatch_action(Box::new(UpdateAction::Check), cx);
                            }),
                    )
                })
                .child(
                    Button::new("title-settings")
                        .ghost()
                        .icon(IconName::Settings)
                        .tooltip(i18n_sidebar(cx, "preferences"))
                        .on_click(|_, _, cx| crate::views::open_settings_window(cx)),
                )
                .child(
                    Button::new("title-about")
                        .ghost()
                        .icon(IconName::Info)
                        .on_click(|_, _, cx| open_about_window(cx)),
                )
                .child(
                    Button::new("title-diag")
                        .ghost()
                        .icon(IconName::ArrowDown)
                        .on_click(|_, window, cx| {
                            window.dispatch_action(Box::new(DiagnosticsAction::Export), cx);
                        }),
                ),
        )
    }
}
