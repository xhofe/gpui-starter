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
use crate::helpers::{card_background, now_datetime};
use crate::states::i18n_home;
use gpui::{App, Window, prelude::*};
use gpui_kit::component::{ActiveTheme, h_flex, label::Label, v_flex};
use gpui_starter_ui::Card;

pub struct Home;

impl Render for Home {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .p_8()
            .gap_4()
            .child(
                Label::new(i18n_home(cx, "title"))
                    .text_xl()
                    .font_weight(gpui::FontWeight::BOLD),
            )
            .child(
                Label::new(i18n_home(cx, "body"))
                    .text_color(cx.theme().muted_foreground)
                    .text_sm(),
            )
            .child(
                h_flex().gap_4().child(
                    Card::new("home-welcome")
                        .title(APP_NAME)
                        .description(i18n_home(cx, "card_body"))
                        .bg(card_background(cx)),
                ),
            )
            .child(
                Label::new(now_datetime())
                    .text_sm()
                    .text_color(cx.theme().muted_foreground),
            )
    }
}

impl Home {
    pub fn new(_cx: &mut App) -> Self {
        Self
    }
}
