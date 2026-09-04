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

use crate::helpers::with_app_identity;
use gpui::{
    AnyWindowHandle, App, AppContext, DisplayId, Entity, FocusHandle, Focusable, Global, KeyDownEvent, Window,
    WindowOptions, div, prelude::*,
};
use gpui_kit::component::Root;
use std::{any::TypeId, collections::HashMap};

pub fn active_window_display(cx: &mut App) -> Option<DisplayId> {
    let handle = cx.active_window()?;
    handle
        .update(cx, |_, window, cx| window.display(cx).map(|d| d.id()))
        .ok()
        .flatten()
}

struct SecondaryWindowRegistry(HashMap<TypeId, AnyWindowHandle>);

impl Global for SecondaryWindowRegistry {}

impl SecondaryWindowRegistry {
    fn get(cx: &mut App) -> &mut Self {
        if cx.try_global::<Self>().is_none() {
            cx.set_global(Self(HashMap::new()));
        }
        cx.global_mut::<Self>()
    }
}

struct SecondaryWindow<V: Render + 'static> {
    focus_handle: FocusHandle,
    content: Entity<V>,
}

impl<V: Render + 'static> Focusable for SecondaryWindow<V> {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl<V: Render + 'static> Render for SecondaryWindow<V> {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|_, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    window.remove_window();
                    cx.stop_propagation();
                }
            }))
            .child(self.content.clone())
    }
}

pub fn open_secondary_window<V: Render + 'static>(
    options: WindowOptions,
    cx: &mut App,
    build: impl FnOnce(&mut Window, &mut App) -> Entity<V>,
) {
    let type_id = TypeId::of::<V>();
    if let Some(existing) = SecondaryWindowRegistry::get(cx).0.get(&type_id).copied()
        && existing.update(cx, |_, window, _| window.activate_window()).is_ok()
    {
        return;
    }
    let opened = cx.open_window(with_app_identity(options), |window, cx| {
        let content = build(window, cx);
        let view = cx.new(|cx| SecondaryWindow {
            focus_handle: cx.focus_handle(),
            content,
        });
        cx.new(|cx| Root::new(view, window, cx))
    });
    if let Ok(handle) = opened {
        SecondaryWindowRegistry::get(cx).0.insert(type_id, handle.into());
    }
}
