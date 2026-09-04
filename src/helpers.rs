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

mod action;
mod app_identity;
mod color;
mod crash;
mod datetime;
mod diagnostics;
mod env;
mod font;
mod fs;
mod keybindings;
mod logger;
mod proxy;
mod single_instance;
mod updater;
mod zip;

pub use action::*;
pub use app_identity::with_app_identity;
pub use color::card_background;
pub use crash::{CrashContext, CrashReport, install_panic_hook, take_pending_crash};
pub use datetime::*;
pub use diagnostics::{DiagnosticsInput, export_diagnostics};
pub use env::is_development;
pub use font::*;
pub use fs::*;
pub use keybindings::{ensure_keybindings_file, keybinding_overrides, load_keybinding_overrides};
pub use logger::{init_logger, logs_dir};
pub use proxy::{is_valid_proxy_setting, set_configured_proxy};
pub use single_instance::{
    InstanceMessage, InstanceRole, claim_instance, instance_messages, post_instance_message, release_instance,
    take_instance_server,
};
#[cfg(target_os = "macos")]
pub use updater::relaunch;
pub use updater::{
    Delivery, UpdateInfo, download_and_verify, fetch_latest_release, focus_installer_ui, install_update,
    installer_requires_quit,
};
