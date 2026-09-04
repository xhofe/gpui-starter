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

use crate::states::{GlobalEvent, GlobalStore, Route};
use crate::views::{home::Home, todos::Todos};
use gpui::{Entity, Subscription, Window, prelude::*};
use gpui_kit::component::v_flex;

pub struct Content {
    home: Entity<Home>,
    todos: Entity<Todos>,
    _sub: Subscription,
}

impl Content {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let store = cx.global::<GlobalStore>().state();
        let sub = cx.subscribe(&store, |_, _, event, cx| {
            if matches!(event, GlobalEvent::RouteChanged) {
                cx.notify();
            }
        });
        Self {
            home: cx.new(|cx| Home::new(cx)),
            todos: cx.new(|cx| Todos::new(window, cx)),
            _sub: sub,
        }
    }
}

impl Render for Content {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let route = cx.global::<GlobalStore>().read(cx).route();
        v_flex().size_full().min_h_0().child(match route {
            Route::Home | Route::Settings => self.home.clone().into_any_element(),
            Route::Todos => self.todos.clone().into_any_element(),
        })
    }
}
