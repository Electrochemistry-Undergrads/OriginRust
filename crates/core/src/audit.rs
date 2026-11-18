//! Tools > Protection > Audit Log (T131). Built from Origin's documented behaviour, **not measured**
//! (Origin's COM automation was unregistered on the test machine on 2026-11-16; `docs/spec/gui-look.md`,
//! "Audit Log").
//!
//! A project can keep a log of the changes made to it: the time (UTC) and the name of each change, as the
//! undo history names it, plus "Undo ..." and "Redo ...". The log is off until it is turned on for the
//! project; it is saved with the project and, unlike the project's content, it is not taken back by Undo
//! (the [`crate::history::Document`] carries it forward).

use serde::{Deserialize, Serialize};

/// One logged change.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Seconds since 1970-01-01 00:00:00 UTC.
    pub time: u64,
    pub change: String,
}

/// The project's audit log (in [`crate::Project::audit`]; `None` there means it is off).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditLog {
    #[serde(default)]
    pub entries: Vec<AuditEntry>,
}

/// Seconds since the Unix epoch now (0 if the clock is before it).
pub fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// `time` as `YYYY-MM-DD HH:MM:SS` (UTC).
pub fn format_time(time: u64) -> String {
    let days = (time / 86_400) as i64;
    let secs = time % 86_400;
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}", secs / 3600, secs / 60 % 60, secs % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_are_written_as_utc_dates() {
        assert_eq!(format_time(0), "1970-01-01 00:00:00");
        assert_eq!(format_time(951_782_400), "2000-02-29 00:00:00");
        assert_eq!(format_time(1_794_312_000 + 3_723), "2026-11-10 13:02:03");
    }
}
