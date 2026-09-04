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

use crate::helpers::keybinding_overrides;
use gpui::Action;
use gpui::KeyBinding;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Clone, Copy, PartialEq, Debug, Deserialize, JsonSchema, Action)]
pub enum MemuAction {
    Quit,
    About,
    Close,
    OpenLogs,
}

#[derive(Clone, Copy, PartialEq, Debug, Deserialize, JsonSchema, Action)]
pub enum WorkspaceTabAction {
    Select(usize),
    New,
}

#[derive(Clone, Copy, PartialEq, Debug, Deserialize, JsonSchema, Action)]
pub enum PaletteAction {
    Toggle,
}

#[derive(Clone, Copy, PartialEq, Debug, Deserialize, JsonSchema, Action)]
pub enum ShortcutsAction {
    Toggle,
}

#[derive(Clone, Copy, PartialEq, Debug, Deserialize, JsonSchema, Action)]
pub enum ZoomAction {
    In,
    Out,
    Reset,
}

#[derive(Clone, Copy, PartialEq, Debug, Deserialize, JsonSchema, Action)]
pub enum WindowAction {
    Minimize,
    Zoom,
    ToggleFullscreen,
}

#[derive(Clone, Copy, PartialEq, Debug, Deserialize, JsonSchema, Action)]
pub enum DiagnosticsAction {
    Export,
}

#[derive(Clone, Copy, PartialEq, Debug, Deserialize, JsonSchema, Action)]
pub enum UpdateAction {
    Check,
    OpenPrompt,
}

#[derive(Clone, Copy, PartialEq, Debug, Deserialize, JsonSchema, Action)]
pub enum SettingsAction {
    Open,
}

pub fn humanize_keystroke(keystroke: &str) -> String {
    let parts = keystroke.split('-');
    let mut display_text = String::new();

    #[cfg(target_os = "macos")]
    let separator = "";
    #[cfg(not(target_os = "macos"))]
    let separator = "+";

    for (i, part) in parts.enumerate() {
        if i > 0 {
            display_text.push_str(separator);
        }
        let symbol = match part {
            "cmd" | "secondary" => {
                #[cfg(target_os = "macos")]
                {
                    "⌘"
                }
                #[cfg(not(target_os = "macos"))]
                {
                    "Ctrl"
                }
            }
            "ctrl" => {
                #[cfg(target_os = "macos")]
                {
                    "⌃"
                }
                #[cfg(not(target_os = "macos"))]
                {
                    "Ctrl"
                }
            }
            "alt" => {
                #[cfg(target_os = "macos")]
                {
                    "⌥"
                }
                #[cfg(not(target_os = "macos"))]
                {
                    "Alt"
                }
            }
            "shift" => {
                #[cfg(target_os = "macos")]
                {
                    "⇧"
                }
                #[cfg(not(target_os = "macos"))]
                {
                    "Shift"
                }
            }
            other => other,
        };
        display_text.push_str(symbol);
    }
    display_text
}

pub struct HotKey {
    pub id: &'static str,
    pub default: &'static str,
    pub reference: Option<(&'static str, &'static str)>,
    bind: fn(&str) -> KeyBinding,
}

impl HotKey {
    pub fn effective(&self) -> &str {
        keybinding_overrides()
            .get(self.id)
            .map(String::as_str)
            .unwrap_or(self.default)
    }
}

const GROUP_GENERAL: &str = "group_general";
const GROUP_NAVIGATION: &str = "group_navigation";

static HOT_KEYS: &[HotKey] = &[
    HotKey {
        id: "command_palette",
        default: "secondary-k",
        reference: Some((GROUP_GENERAL, "command_palette")),
        bind: |keystroke: &str| KeyBinding::new(keystroke, PaletteAction::Toggle, None),
    },
    HotKey {
        id: "keyboard_shortcuts",
        default: "secondary-/",
        reference: Some((GROUP_GENERAL, "keyboard_shortcuts")),
        bind: |keystroke: &str| KeyBinding::new(keystroke, ShortcutsAction::Toggle, None),
    },
    HotKey {
        id: "zoom_in",
        default: "secondary-=",
        reference: Some((GROUP_GENERAL, "zoom_in")),
        bind: |keystroke: &str| KeyBinding::new(keystroke, ZoomAction::In, None),
    },
    HotKey {
        id: "zoom_out",
        default: "secondary--",
        reference: Some((GROUP_GENERAL, "zoom_out")),
        bind: |keystroke: &str| KeyBinding::new(keystroke, ZoomAction::Out, None),
    },
    HotKey {
        id: "zoom_reset",
        default: "secondary-0",
        reference: Some((GROUP_GENERAL, "zoom_reset")),
        bind: |keystroke: &str| KeyBinding::new(keystroke, ZoomAction::Reset, None),
    },
    HotKey {
        id: "quit",
        default: "secondary-q",
        reference: Some((GROUP_GENERAL, "quit")),
        bind: |keystroke: &str| KeyBinding::new(keystroke, MemuAction::Quit, None),
    },
    HotKey {
        id: "settings",
        default: "secondary-,",
        reference: Some((GROUP_GENERAL, "settings")),
        bind: |keystroke: &str| KeyBinding::new(keystroke, SettingsAction::Open, None),
    },
    HotKey {
        id: "new_tab",
        default: "secondary-t",
        reference: Some((GROUP_NAVIGATION, "new_tab")),
        bind: |keystroke: &str| KeyBinding::new(keystroke, WorkspaceTabAction::New, None),
    },
    HotKey {
        id: "close_window",
        default: "secondary-w",
        reference: None,
        bind: |keystroke: &str| KeyBinding::new(keystroke, MemuAction::Close, None),
    },
];

pub fn hot_key_table() -> &'static [HotKey] {
    HOT_KEYS
}

pub struct ShortcutGroup {
    pub title_key: &'static str,
    pub items: Vec<(String, &'static str)>,
}

pub fn shortcut_reference() -> Vec<ShortcutGroup> {
    let mut groups = vec![
        ShortcutGroup {
            title_key: GROUP_GENERAL,
            items: Vec::new(),
        },
        ShortcutGroup {
            title_key: GROUP_NAVIGATION,
            items: Vec::new(),
        },
    ];
    for hot_key in HOT_KEYS {
        let Some((group, desc_key)) = hot_key.reference else {
            continue;
        };
        if let Some(group) = groups.iter_mut().find(|candidate| candidate.title_key == group) {
            group.items.push((hot_key.effective().to_string(), desc_key));
        }
    }
    groups
}

pub fn new_hot_keys() -> Vec<KeyBinding> {
    let mut keys: Vec<KeyBinding> = HOT_KEYS
        .iter()
        .map(|hot_key| (hot_key.bind)(hot_key.effective()))
        .collect();
    for i in 0..8 {
        keys.push(KeyBinding::new(
            &format!("secondary-{}", i + 1),
            WorkspaceTabAction::Select(i),
            None,
        ));
    }
    keys
}
