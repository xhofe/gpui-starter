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

use crate::helpers::format_unix_secs;
use crate::states::{dialog_button_props, i18n_common, i18n_todos};
use gpui::{App, Entity, Subscription, Window, div, prelude::*, px};
use gpui_kit::component::{
    ActiveTheme, IconName,
    button::{Button, ButtonVariants},
    checkbox::Checkbox,
    h_flex,
    input::{Input, InputEvent, InputState},
    label::Label,
    scroll::ScrollableElement,
    v_flex,
};
use gpui_starter_db::{Todo, add_todo, delete_todo, list_todos, set_todo_done};
use gpui_starter_ui::Dialog;
use tracing::error;

pub struct Todos {
    items: Vec<Todo>,
    input: Entity<InputState>,
    _input_sub: Subscription,
}

impl Todos {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(i18n_todos(cx, "placeholder")));
        let _input_sub = cx.subscribe_in(&input, window, |this, _input, event, window, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                this.add(window, cx);
            }
        });
        let items = list_todos().unwrap_or_else(|e| {
            error!(error = %e, "list todos");
            Vec::new()
        });
        Self {
            items,
            input,
            _input_sub,
        }
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        match list_todos() {
            Ok(items) => self.items = items,
            Err(e) => error!(error = %e, "list todos"),
        }
        cx.notify();
    }

    fn add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let title = self.input.read(cx).value().to_string();
        match add_todo(title) {
            Ok(_) => {
                self.input.update(cx, |input, cx| input.set_value("", window, cx));
                self.reload(cx);
            }
            Err(e) => error!(error = %e, "add todo"),
        }
    }

    fn toggle(&mut self, id: String, done: bool, cx: &mut Context<Self>) {
        if let Err(e) = set_todo_done(&id, done) {
            error!(error = %e, "toggle todo");
        }
        self.reload(cx);
    }

    fn confirm_delete(&self, id: String, title: String, window: &mut Window, cx: &mut Context<Self>) {
        let this = cx.entity().downgrade();
        Dialog::new(i18n_todos(cx, "delete_title"))
            .message(t_delete_body(cx, &title))
            .button_props(dialog_button_props(cx))
            .ok_text(i18n_common(cx, "delete"))
            .on_ok(move |_, _, cx| {
                if let Some(this) = this.upgrade() {
                    this.update(cx, |this, cx| {
                        if let Err(e) = delete_todo(&id) {
                            error!(error = %e, "delete todo");
                        }
                        this.reload(cx);
                    });
                }
                true
            })
            .open(window, cx);
        let _ = title;
    }
}

fn t_delete_body(cx: &App, title: &str) -> String {
    rust_i18n::t!(
        "todos.delete_body",
        title = title,
        locale = cx.global::<crate::states::GlobalStore>().read(cx).locale()
    )
    .to_string()
}

impl Render for Todos {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let empty = self.items.is_empty();
        v_flex()
            .size_full()
            .p_6()
            .gap_4()
            .child(
                Label::new(i18n_todos(cx, "title"))
                    .text_xl()
                    .font_weight(gpui::FontWeight::BOLD),
            )
            .child(
                h_flex()
                    .gap_2()
                    .w_full()
                    .child(div().flex_1().h(px(32.)).child(Input::new(&self.input)))
                    .child(
                        Button::new("todo-add")
                            .primary()
                            .label(i18n_todos(cx, "add"))
                            .on_click(cx.listener(|this, _, window, cx| this.add(window, cx))),
                    ),
            )
            .child(if empty {
                Label::new(i18n_todos(cx, "empty"))
                    .text_color(cx.theme().muted_foreground)
                    .into_any_element()
            } else {
                v_flex()
                    .gap_1()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .children(self.items.iter().map(|todo| {
                        let toggle_id = todo.id.clone();
                        let delete_id = todo.id.clone();
                        let delete_title = todo.title.clone();
                        let done = todo.done;
                        let created = format_unix_secs(todo.created_at).unwrap_or_default();
                        h_flex()
                            .id(todo.id.clone())
                            .gap_2()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .hover(|s| s.bg(cx.theme().secondary))
                            .child(
                                Checkbox::new(format!("todo-done-{}", todo.id))
                                    .checked(done)
                                    .on_click(cx.listener(move |this, checked, _, cx| {
                                        this.toggle(toggle_id.clone(), *checked, cx);
                                    })),
                            )
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .child(
                                        Label::new(todo.title.clone())
                                            .when(done, |this| this.text_color(cx.theme().muted_foreground)),
                                    )
                                    .child(Label::new(created).text_xs().text_color(cx.theme().muted_foreground)),
                            )
                            .child(
                                Button::new(format!("todo-del-{}", todo.id))
                                    .ghost()
                                    .icon(IconName::Delete)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.confirm_delete(delete_id.clone(), delete_title.clone(), window, cx);
                                    })),
                            )
                    }))
                    .into_any_element()
            })
    }
}
