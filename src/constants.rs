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

use gpui::{Pixels, px};

/// User-facing application name (window title, menus, About).
pub const APP_NAME: &str = "GPUI Starter";

/// Freedesktop / Wayland `app_id` for AppImage and tarball installs.
pub const APP_ID: &str = "gpui-starter";

/// Bundle identifier — keep in lockstep with `[package.metadata.bundle]`.
pub const BUNDLE_ID: &str = "com.example.gpui-starter";

pub fn linux_app_id() -> String {
    std::env::var("FLATPAK_ID")
        .ok()
        .filter(|id| !id.is_empty())
        .unwrap_or_else(|| APP_ID.to_string())
}

pub const SIDEBAR_WIDTH: Pixels = px(180.0);
pub const SIDEBAR_COLLAPSED_WIDTH: Pixels = px(52.0);
pub const WORKSPACE_TAB_BAR_HEIGHT: Pixels = px(30.0);
