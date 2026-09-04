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

mod about;
mod command_palette;
mod content;
mod home;
mod secondary_window;
mod settings;
mod shortcuts_overlay;
mod sidebar;
mod title_bar;
mod todos;
mod update_dialog;

pub use about::open_about_window;
pub use command_palette::CommandPalette;
pub use content::Content;
pub use settings::open_settings_window;
pub use shortcuts_overlay::ShortcutsOverlay;
pub use sidebar::Sidebar;
pub use title_bar::TitleBar;
pub use update_dialog::{DialogCallback, UpdateDialog};
