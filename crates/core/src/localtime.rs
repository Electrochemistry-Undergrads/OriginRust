//! The local time zone offset, for `now()` and `today()` in column formulas (Origin uses local time).

/// Seconds the local time is ahead of UTC right now (negative west of Greenwich, daylight saving
/// time included). Zero when the system gives no answer.
pub fn local_utc_offset_seconds() -> i64 {
    imp::offset()
}

#[cfg(windows)]
mod imp {
    use windows_sys::Win32::System::Time::{GetTimeZoneInformation, TIME_ZONE_INFORMATION};

    pub fn offset() -> i64 {
        // SAFETY: the structure is plain data and the call only fills it in.
        let (info, id) = unsafe {
            let mut info: TIME_ZONE_INFORMATION = std::mem::zeroed();
            let id = GetTimeZoneInformation(&mut info);
            (info, id)
        };
        const TIME_ZONE_ID_INVALID: u32 = u32::MAX;
        const TIME_ZONE_ID_DAYLIGHT: u32 = 2;
        if id == TIME_ZONE_ID_INVALID {
            return 0;
        }
        // UTC = local + bias (minutes), so the offset east of UTC is the negated bias.
        let extra = if id == TIME_ZONE_ID_DAYLIGHT { info.DaylightBias } else { info.StandardBias };
        -(i64::from(info.Bias) + i64::from(extra)) * 60
    }
}

#[cfg(unix)]
mod imp {
    #[allow(clippy::useless_conversion)] // tm_gmtoff is i64 on macOS and i32 on Linux
    pub fn offset() -> i64 {
        // SAFETY: `localtime_r` writes only into the structure given to it.
        unsafe {
            let now: libc::time_t = libc::time(std::ptr::null_mut());
            let mut tm: libc::tm = std::mem::zeroed();
            if libc::localtime_r(&now, &mut tm).is_null() { 0 } else { i64::from(tm.tm_gmtoff) }
        }
    }
}

#[cfg(not(any(windows, unix)))]
mod imp {
    pub fn offset() -> i64 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_offset_is_a_real_time_zone() {
        let o = local_utc_offset_seconds();
        assert!((-12 * 3600..=14 * 3600).contains(&o), "{o}");
        assert_eq!(o % 900, 0, "zones are whole quarter hours");
    }
}
