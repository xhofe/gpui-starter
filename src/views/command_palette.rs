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

//! ⌘K command palette: fuzzy-ish search over navigation commands.

use crate::helpers::{SettingsAction, ShortcutsAction, UpdateAction, WorkspaceTabAction};
use crate::states::{GlobalStore, Route, i18n_command_palette};
use crate::views::open_about_window;
use gpui::{
    App, Context, Entity, FocusHandle, Focusable, KeyDownEvent, ScrollHandle, Subscription, Window, div, prelude::*, px,
};
use gpui_kit::component::{
    ActiveTheme,
    input::{Input, InputEvent, InputState},
    label::Label,
    v_flex,
};

#[derive(Clone)]
enum PaletteCommand {
    Route(Route),
    Settings,
    About,
    Shortcuts,
    NewTab,
    CheckUpdates,
}

struct PaletteItem {
    label: gpui::SharedString,
    search: String,
    command: PaletteCommand,
}

pub struct CommandPalette {
    open: bool,
    query: Entity<InputState>,
    selected: usize,
    focus_handle: FocusHandle,
    pending_focus: bool,
    pending_restore: bool,
    prev_focus: Option<FocusHandle>,
    scroll_handle: ScrollHandle,
    items: Vec<PaletteItem>,
    _query_sub: Subscription,
}

impl CommandPalette {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query =
            cx.new(|cx| InputState::new(window, cx).placeholder(i18n_command_palette(cx, "search_placeholder")));
        let _query_sub = cx.subscribe_in(&query, window, |this, _input, event, _window, cx| {
            if matches!(event, InputEvent::Change) {
                this.selected = 0;
                cx.notify();
            }
        });
        Self {
            open: false,
            query,
            selected: 0,
            focus_handle: cx.focus_handle(),
            pending_focus: false,
            pending_restore: false,
            prev_focus: None,
            scroll_handle: ScrollHandle::new(),
            items: Vec::new(),
            _query_sub,
        }
    }

    pub fn toggle(&mut self, cx: &mut Context<Self>) {
        self.open = !self.open;
        if self.open {
            self.selected = 0;
            self.pending_focus = true;
            self.scroll_handle.set_offset(gpui::Point::default());
        } else {
            self.pending_restore = true;
        }
        cx.notify();
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open = false;
        self.pending_restore = true;
        window.blur(cx);
        cx.notify();
    }

    fn ranked(&self, cx: &App) -> Vec<usize> {
        let query = self.query.read(cx).value().to_string().to_ascii_lowercase();
        let mut order: Vec<usize> = (0..self.items.len()).collect();
        if !query.is_empty() {
            order.retain(|i| self.items[*i].search.contains(&query));
        }
        order
    }

    fn run(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self.items.get(index) else {
            return;
        };
        match item.command.clone() {
            PaletteCommand::Route(route) => {
                cx.global::<GlobalStore>()
                    .clone()
                    .update(cx, |state, cx| state.go_to(route, cx));
            }
            PaletteCommand::Settings => cx.dispatch_action(&SettingsAction::Open),
            PaletteCommand::About => open_about_window(cx),
            PaletteCommand::Shortcuts => cx.dispatch_action(&ShortcutsAction::Toggle),
            PaletteCommand::NewTab => cx.dispatch_action(&WorkspaceTabAction::New),
            PaletteCommand::CheckUpdates => cx.dispatch_action(&UpdateAction::Check),
        }
        self.close(window, cx);
    }
}

impl Focusable for CommandPalette {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for CommandPalette {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.open {
            if self.pending_restore {
                self.pending_restore = false;
                if let Some(focus) = self.prev_focus.take() {
                    focus.focus(window, cx);
                } else {
                    window.blur(cx);
                }
            }
            return div().into_any_element();
        }

        self.items = vec![
            item(i18n_command_palette(cx, "home"), PaletteCommand::Route(Route::Home)),
            item(i18n_command_palette(cx, "todos"), PaletteCommand::Route(Route::Todos)),
            item(i18n_command_palette(cx, "settings"), PaletteCommand::Settings),
            item(i18n_command_palette(cx, "about"), PaletteCommand::About),
            item(i18n_command_palette(cx, "shortcuts"), PaletteCommand::Shortcuts),
            item(i18n_command_palette(cx, "new_tab"), PaletteCommand::NewTab),
            item(i18n_command_palette(cx, "check_updates"), PaletteCommand::CheckUpdates),
        ];
        let order = self.ranked(cx);
        if self.selected >= order.len() {
            self.selected = 0;
        }

        if self.pending_focus {
            self.pending_focus = false;
            self.prev_focus = window.focused(cx);
            self.query.update(cx, |input, cx| {
                input.set_value("", window, cx);
                input.focus(window, cx);
            });
        }

        let theme = cx.theme();
        let panel_bg = theme.background;
        let border = theme.border;
        let muted = theme.muted_foreground;
        let fg = theme.foreground;
        let selected_bg = fg.alpha(0.1);
        let rows: Vec<_> = order
            .iter()
            .enumerate()
            .map(|(visible, &item_ix)| {
                let label = self.items[item_ix].label.clone();
                let is_selected = visible == self.selected;
                div()
                    .id(("palette-row", item_ix))
                    .w_full()
                    .px_3()
                    .py_1p5()
                    .when(is_selected, |this| this.bg(selected_bg))
                    .on_click(cx.listener(move |this, _, window, cx| this.run(item_ix, window, cx)))
                    .child(Label::new(label).text_sm().text_color(fg))
            })
            .collect();

        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .justify_center()
            .items_start()
            .bg(gpui::hsla(0., 0., 0., 0.4))
            .track_focus(&self.focus_handle)
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, window, cx| this.close(window, cx)),
            )
            .capture_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                match event.keystroke.key.as_str() {
                    "escape" => {
                        this.close(window, cx);
                        cx.stop_propagation();
                    }
                    "enter" => {
                        let order = this.ranked(cx);
                        if let Some(&ix) = order.get(this.selected) {
                            this.run(ix, window, cx);
                        }
                        cx.stop_propagation();
                    }
                    "up" => {
                        this.selected = this.selected.saturating_sub(1);
                        cx.notify();
                        cx.stop_propagation();
                    }
                    "down" => {
                        let len = this.ranked(cx).len();
                        if len > 0 {
                            this.selected = (this.selected + 1).min(len - 1);
                        }
                        cx.notify();
                        cx.stop_propagation();
                    }
                    _ => {}
                }
            }))
            .child(
                v_flex()
                    .mt(px(96.))
                    .w(px(480.))
                    .max_h(px(420.))
                    .bg(panel_bg)
                    .border_1()
                    .border_color(border)
                    .rounded(theme.radius_lg)
                    .shadow_lg()
                    .overflow_hidden()
                    .on_mouse_down(gpui::MouseButton::Left, |_, _, cx: &mut gpui::App| {
                        cx.stop_propagation();
                    })
                    .child(div().px_3().py_2().child(Input::new(&self.query)))
                    .child(
                        v_flex()
                            .id("palette-list")
                            .w_full()
                            .max_h(px(320.))
                            .overflow_y_scroll()
                            .track_scroll(&self.scroll_handle)
                            .children(rows)
                            .when(order.is_empty(), |this| {
                                this.child(
                                    div().px_3().py_2().child(
                                        Label::new(i18n_command_palette(cx, "empty"))
                                            .text_sm()
                                            .text_color(muted),
                                    ),
                                )
                            }),
                    ),
            )
            .into_any_element()
    }
}

fn item(label: gpui::SharedString, command: PaletteCommand) -> PaletteItem {
    PaletteItem {
        search: label.to_string().to_ascii_lowercase(),
        label,
        command,
    }
}
