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

use snafu::Snafu;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(transparent)]
    Db { source: gpui_starter_db::error::Error },
    #[snafu(display("Invalid: {message}"))]
    Invalid { message: String },
    #[snafu(display("IO error: {source}"))]
    Io { source: std::io::Error },
    #[snafu(display("TOML error: {source}"))]
    TomlDe { source: toml::de::Error },
    #[snafu(display("TOML error: {source}"))]
    TomlSer { source: toml::ser::Error },
    #[snafu(display("JSON error: {source}"))]
    Json { source: serde_json::Error },
}

impl From<std::io::Error> for Error {
    fn from(source: std::io::Error) -> Self {
        Error::Io { source }
    }
}
impl From<toml::de::Error> for Error {
    fn from(source: toml::de::Error) -> Self {
        Error::TomlDe { source }
    }
}
impl From<toml::ser::Error> for Error {
    fn from(source: toml::ser::Error) -> Self {
        Error::TomlSer { source }
    }
}
impl From<serde_json::Error> for Error {
    fn from(source: serde_json::Error) -> Self {
        Error::Json { source }
    }
}
