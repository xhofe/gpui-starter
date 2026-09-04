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

//! Process-wide date / time rendering preference.
//!
//! Settings "Time zone" / "Date format" choice applies everywhere at once.
//! The preference is mirrored into a process-wide slot (like the HTTP
//! proxy) because several call sites format on background threads or in
//! pure helpers with no `App` in reach. File-name stamps and diagnostics
//! deliberately stay on their own fixed formats.

use chrono::{DateTime, Local, TimeZone, Utc};
use std::sync::RwLock;

/// Which zone timestamps are rendered in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TimeZonePref {
    #[default]
    Local,
    Utc,
}

impl TimeZonePref {
    pub const ALL: [TimeZonePref; 2] = [TimeZonePref::Local, TimeZonePref::Utc];

    pub fn from_name(name: &str) -> Self {
        match name {
            "utc" => TimeZonePref::Utc,
            _ => TimeZonePref::Local,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            TimeZonePref::Local => "local",
            TimeZonePref::Utc => "utc",
        }
    }

    /// Short label for the Settings select ("Local" / "UTC").
    pub fn label(self) -> &'static str {
        match self {
            TimeZonePref::Local => "Local",
            TimeZonePref::Utc => "UTC",
        }
    }
}

/// One selectable date + time layout. `id` is what `gpui-starter.toml` stores.
#[derive(Debug, PartialEq, Eq)]
pub struct DateFormat {
    pub id: &'static str,
    /// strftime pattern for a full date + time.
    pub pattern: &'static str,
}

/// The selectable layouts, in the order the Settings select lists them.
/// The first entry is the default.
pub const DATE_FORMATS: &[DateFormat] = &[
    DateFormat {
        id: "iso",
        pattern: "%Y-%m-%d %H:%M:%S",
    },
    DateFormat {
        id: "rfc3339",
        pattern: "%Y-%m-%dT%H:%M:%S%:z",
    },
    DateFormat {
        id: "dmy",
        pattern: "%d/%m/%Y %H:%M:%S",
    },
    DateFormat {
        id: "dmy_dot",
        pattern: "%d.%m.%Y %H:%M:%S",
    },
    DateFormat {
        id: "mdy",
        pattern: "%m/%d/%Y %I:%M:%S %p",
    },
];

pub const DEFAULT_DATE_FORMAT: &str = "iso";

/// Resolve a stored id, falling back to the default for an unknown one
/// (an older / hand-edited config).
pub fn date_format_by_id(id: &str) -> &'static DateFormat {
    DATE_FORMATS
        .iter()
        .find(|format| format.id == id)
        .or_else(|| DATE_FORMATS.iter().find(|format| format.id == DEFAULT_DATE_FORMAT))
        .unwrap_or(&DATE_FORMATS[0])
}

/// A fixed instant rendered in the layout, for the Settings select labels:
/// the sample explains the format better than any name would.
pub fn date_format_sample(format: &DateFormat) -> String {
    // 2026-03-04 15:06:07 UTC — every field is distinct, so the day / month
    // order and the 12-hour marker are visible at a glance.
    match Utc.with_ymd_and_hms(2026, 3, 4, 15, 6, 7).single() {
        Some(sample) => sample.format(format.pattern).to_string(),
        None => format.pattern.to_string(),
    }
}

struct DateTimePrefs {
    zone: TimeZonePref,
    format: &'static DateFormat,
}

static PREFS: RwLock<DateTimePrefs> = RwLock::new(DateTimePrefs {
    zone: TimeZonePref::Local,
    format: &DATE_FORMATS[0],
});

/// Mirror the persisted preference into the process-wide slot. Called at
/// startup and whenever the Settings page changes either value.
pub fn set_datetime_prefs(zone: TimeZonePref, format_id: &str) {
    let format = date_format_by_id(format_id);
    let mut prefs = PREFS.write().unwrap_or_else(|e| e.into_inner());
    prefs.zone = zone;
    prefs.format = format;
}

fn current() -> (TimeZonePref, &'static DateFormat) {
    let prefs = PREFS.read().unwrap_or_else(|e| e.into_inner());
    (prefs.zone, prefs.format)
}

fn render<Tz: TimeZone>(dt: &DateTime<Tz>, zone: TimeZonePref, pattern: &str) -> String {
    match zone {
        TimeZonePref::Local => dt.with_timezone(&Local).format(pattern).to_string(),
        TimeZonePref::Utc => dt.with_timezone(&Utc).format(pattern).to_string(),
    }
}

/// Full date + time in the configured zone and layout.
pub fn format_datetime<Tz: TimeZone>(dt: &DateTime<Tz>) -> String {
    let (zone, format) = current();
    render(dt, zone, format.pattern)
}

/// Unix seconds → configured date + time; `None` when out of chrono's range.
pub fn format_unix_secs(ts: i64) -> Option<String> {
    DateTime::from_timestamp(ts, 0).map(|dt| format_datetime(&dt))
}

/// The current date + time in the configured zone and layout.
pub fn now_datetime() -> String {
    format_datetime(&Utc::now())
}

/// Unix seconds, or 0 if the system clock is before the epoch.
pub fn unix_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard};

    /// The prefs slot is process-wide, so the tests that set it serialise.
    static SERIAL: Mutex<()> = Mutex::new(());

    fn lock() -> MutexGuard<'static, ()> {
        SERIAL.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn utc_zone_and_layout_apply_to_every_helper() {
        let _guard = lock();
        set_datetime_prefs(TimeZonePref::Utc, "rfc3339");
        // 2026-03-04T15:06:07Z
        let ts = 1_772_636_767;
        assert_eq!(format_unix_secs(ts).as_deref(), Some("2026-03-04T15:06:07+00:00"));
        set_datetime_prefs(TimeZonePref::Local, DEFAULT_DATE_FORMAT);
    }

    #[test]
    fn unknown_format_id_falls_back_to_the_default_layout() {
        let _guard = lock();
        set_datetime_prefs(TimeZonePref::Utc, "no-such-layout");
        assert_eq!(format_unix_secs(0).as_deref(), Some("1970-01-01 00:00:00"));
        set_datetime_prefs(TimeZonePref::Local, DEFAULT_DATE_FORMAT);
    }

    #[test]
    fn samples_show_the_field_order() {
        assert_eq!(date_format_sample(date_format_by_id("iso")), "2026-03-04 15:06:07");
        assert_eq!(date_format_sample(date_format_by_id("dmy")), "04/03/2026 15:06:07");
        assert_eq!(date_format_sample(date_format_by_id("mdy")), "03/04/2026 03:06:07 PM");
        assert_eq!(TimeZonePref::from_name("utc"), TimeZonePref::Utc);
        assert_eq!(TimeZonePref::from_name("anything"), TimeZonePref::Local);
    }
}
