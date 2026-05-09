//! ISO 8601 date/time validation and parsing.
//!
//! Validates date, datetime, and duration strings against ISO 8601 format.

/// Validate an ISO 8601 date string (YYYY-MM-DD).
pub fn is_valid_date(s: &str) -> bool {
    if s.len() != 10 {
        return false;
    }
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return false;
    }
    let year: u32 = match parts[0].parse() {
        Ok(v) => v,
        Err(_) => return false,
    };
    let month: u32 = match parts[1].parse() {
        Ok(v) => v,
        Err(_) => return false,
    };
    let day: u32 = match parts[2].parse() {
        Ok(v) => v,
        Err(_) => return false,
    };
    if !(1..=9999).contains(&year) || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return false;
    }
    // Basic month-day validation
    let max_day = match month {
        2 => {
            if year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400)) {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    day <= max_day
}

/// Validate an ISO 8601 datetime string.
/// Accepts: `YYYY-MM-DDTHH:MM:SSZ` or `YYYY-MM-DDTHH:MM:SS+HH:MM`
pub fn is_valid_datetime(s: &str) -> bool {
    // Must contain 'T' separator
    let Some(t_pos) = s.find('T') else {
        return false;
    };
    let date_part = &s[..t_pos];
    if !is_valid_date(date_part) {
        return false;
    }

    let time_part = &s[t_pos + 1..];
    // Strip timezone
    let time_only = if let Some(stripped) = time_part.strip_suffix('Z') {
        stripped
    } else if let Some(plus) = time_part.rfind('+') {
        if plus > 5 {
            &time_part[..plus]
        } else {
            return false;
        }
    } else if let Some(minus) = time_part.rfind('-') {
        if minus > 5 {
            &time_part[..minus]
        } else {
            return false;
        }
    } else {
        time_part // no timezone (local time)
    };

    let parts: Vec<&str> = time_only.split(':').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return false;
    }
    let hour: u32 = match parts[0].parse() {
        Ok(v) => v,
        Err(_) => return false,
    };
    let min: u32 = match parts[1].parse() {
        Ok(v) => v,
        Err(_) => return false,
    };
    if hour > 23 || min > 59 {
        return false;
    }
    if parts.len() == 3 {
        // Seconds may have fractional part
        let sec_str = parts[2].split('.').next().unwrap_or("0");
        let sec: u32 = match sec_str.parse() {
            Ok(v) => v,
            Err(_) => return false,
        };
        if sec > 59 {
            return false;
        }
    }
    true
}

/// Validate an ISO 8601 duration string (e.g., P1Y2M3DT4H5M6S).
pub fn is_valid_duration(s: &str) -> bool {
    if !s.starts_with('P') || s.len() < 2 {
        return false;
    }
    let rest = &s[1..];
    let (date_part, time_part) = if let Some(t_pos) = rest.find('T') {
        (&rest[..t_pos], Some(&rest[t_pos + 1..]))
    } else {
        (rest, None)
    };

    // Validate date part: digits followed by Y, M, or D
    if !date_part.is_empty() && !validate_duration_part(date_part, &['Y', 'M', 'D']) {
        return false;
    }

    // Validate time part: digits followed by H, M, or S
    if let Some(tp) = time_part {
        if tp.is_empty() || !validate_duration_part(tp, &['H', 'M', 'S']) {
            return false;
        }
    }

    true
}

fn validate_duration_part(s: &str, valid_suffixes: &[char]) -> bool {
    let mut num_buf = String::new();
    for ch in s.chars() {
        if ch.is_ascii_digit() || ch == '.' {
            num_buf.push(ch);
        } else if valid_suffixes.contains(&ch) {
            if num_buf.is_empty() {
                return false;
            }
            num_buf.clear();
        } else {
            return false;
        }
    }
    num_buf.is_empty() // all digits should be consumed by a suffix
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Dates ──

    #[test]
    fn valid_dates() {
        assert!(is_valid_date("2026-05-08"));
        assert!(is_valid_date("2000-01-01"));
        assert!(is_valid_date("2024-02-29")); // leap year
    }

    #[test]
    fn invalid_dates() {
        assert!(!is_valid_date("2026-13-01")); // month 13
        assert!(!is_valid_date("2026-02-30")); // feb 30
        assert!(!is_valid_date("2023-02-29")); // not leap year
        assert!(!is_valid_date("not-a-date"));
        assert!(!is_valid_date("2026-5-8")); // missing zero padding
        assert!(!is_valid_date(""));
    }

    // ── Datetimes ──

    #[test]
    fn valid_datetimes() {
        assert!(is_valid_datetime("2026-05-08T10:30:00Z"));
        assert!(is_valid_datetime("2026-05-08T10:30:00+05:00"));
        assert!(is_valid_datetime("2026-05-08T10:30:00-03:00"));
        assert!(is_valid_datetime("2026-05-08T23:59:59Z"));
        assert!(is_valid_datetime("2026-05-08T10:30:00.123Z")); // fractional seconds
        assert!(is_valid_datetime("2026-05-08T10:30Z")); // no seconds
    }

    #[test]
    fn invalid_datetimes() {
        assert!(!is_valid_datetime("2026-05-08")); // no T
        assert!(!is_valid_datetime("2026-05-08T25:00:00Z")); // hour 25
        assert!(!is_valid_datetime("2026-05-08T10:60:00Z")); // min 60
        assert!(!is_valid_datetime("not-a-datetime"));
        assert!(!is_valid_datetime(""));
    }

    // ── Durations ──

    #[test]
    fn valid_durations() {
        assert!(is_valid_duration("P1Y"));
        assert!(is_valid_duration("P1Y2M3D"));
        assert!(is_valid_duration("PT1H"));
        assert!(is_valid_duration("PT1H30M"));
        assert!(is_valid_duration("P1Y2M3DT4H5M6S"));
        assert!(is_valid_duration("P30D"));
        assert!(is_valid_duration("PT0.5S")); // fractional seconds
    }

    #[test]
    fn invalid_durations() {
        assert!(!is_valid_duration("1Y")); // no P prefix
        assert!(!is_valid_duration("P")); // empty
        assert!(!is_valid_duration("")); // empty
        assert!(!is_valid_duration("PT")); // T with no time
        assert!(!is_valid_duration("PXY")); // invalid char
    }
}
