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

use gpui::{Entity, EventEmitter, SharedString, Subscription, Window, prelude::*};
use gpui_kit::component::{
    IndexPath,
    select::{Select as KitSelect, SelectEvent as KitSelectEvent, SelectItem, SelectState},
};

#[derive(Clone)]
struct SelectOption {
    label: SharedString,
    index: usize,
}

impl SelectItem for SelectOption {
    type Value = usize;
    fn title(&self) -> SharedString {
        self.label.clone()
    }
    fn value(&self) -> &Self::Value {
        &self.index
    }
}

pub enum SelectEvent {
    Change(usize),
}

pub struct Select {
    state: Entity<SelectState<Vec<SelectOption>>>,
    _subscription: Subscription,
}

impl EventEmitter<SelectEvent> for Select {}

impl Select {
    pub fn new(items: Vec<String>, selected_index: Option<usize>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::build(items, selected_index, false, window, cx)
    }

    /// Like [`Select::new`] but the dropdown gets a search box — for long
    /// option lists (e.g. installed font families).
    pub fn new_searchable(
        items: Vec<String>,
        selected_index: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self::build(items, selected_index, true, window, cx)
    }

    fn build(
        items: Vec<String>,
        selected_index: Option<usize>,
        searchable: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let options = items
            .into_iter()
            .enumerate()
            .map(|(i, s)| SelectOption {
                label: s.into(),
                index: i,
            })
            .collect::<Vec<_>>();
        let initial = selected_index.map(IndexPath::new);
        let state = cx.new(|cx| {
            let state = SelectState::new(options, initial, window, cx);
            if searchable { state.searchable(true) } else { state }
        });
        let subscription = cx.subscribe_in(
            &state,
            window,
            |_this, _state, event: &KitSelectEvent<Vec<SelectOption>>, _window, cx| {
                let KitSelectEvent::Confirm(value) = event;
                if let Some(index) = *value {
                    cx.emit(SelectEvent::Change(index));
                }
            },
        );
        Self {
            state,
            _subscription: subscription,
        }
    }
}

impl Render for Select {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        KitSelect::new(&self.state)
    }
}
