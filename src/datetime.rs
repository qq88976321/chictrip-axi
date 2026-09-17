//! Calendar helpers. The CLI only needs proleptic Gregorian arithmetic in
//! UTC, which is a few lines, so it carries no date crate.

use crate::error::AxiError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Date {
    pub year: i64,
    pub month: u32,
    pub day: u32,
}

impl Date {
    /// Accepts `YYYY-MM-DD` and `YYYY/MM/DD`; the API speaks the latter.
    pub fn parse(text: &str) -> Result<Date, AxiError> {
        let parts: Vec<&str> = text.split(['-', '/']).collect();
        let bad = || {
            AxiError::usage(format!(
                "invalid date '{text}'; expected YYYY-MM-DD or YYYY/MM/DD"
            ))
        };
        if parts.len() != 3 || parts[0].len() != 4 {
            return Err(bad());
        }
        let year: i64 = parts[0].parse().map_err(|_| bad())?;
        let month: u32 = parts[1].parse().map_err(|_| bad())?;
        let day: u32 = parts[2].parse().map_err(|_| bad())?;
        if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
            return Err(bad());
        }
        Ok(Date { year, month, day })
    }

    pub fn to_api(self) -> String {
        format!("{:04}/{:02}/{:02}", self.year, self.month, self.day)
    }

    fn to_days(self) -> i64 {
        days_from_civil(self.year, self.month, self.day)
    }

    /// Inclusive day count from `self` to `end`; negative when end < self.
    pub fn span_days(self, end: Date) -> i64 {
        end.to_days() - self.to_days() + 1
    }
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Howard Hinnant's days_from_civil: days since 1970-01-01.
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = month as i64;
    let d = day as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Inverse of [`days_from_civil`].
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m as u32, d as u32)
}

pub fn format_utc_date(unix_seconds: i64) -> String {
    let (y, m, d) = civil_from_days(unix_seconds.div_euclid(86_400));
    format!("{y:04}-{m:02}-{d:02}")
}

pub fn format_utc_datetime(unix_seconds: i64) -> String {
    let (y, m, d) = civil_from_days(unix_seconds.div_euclid(86_400));
    let secs = unix_seconds.rem_euclid(86_400);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_parse_in_both_separators_and_render_for_the_api() {
        assert_eq!(Date::parse("2026-10-01").unwrap().to_api(), "2026/10/01");
        assert_eq!(Date::parse("2026/10/01").unwrap().to_api(), "2026/10/01");
        assert!(Date::parse("2026-02-30").is_err());
        assert!(Date::parse("10/01/2026").is_err());
        assert!(Date::parse("2026-13-01").is_err());
    }

    #[test]
    fn spans_are_inclusive() {
        let start = Date::parse("2026-10-01").unwrap();
        assert_eq!(start.span_days(Date::parse("2026-10-01").unwrap()), 1);
        assert_eq!(start.span_days(Date::parse("2026-10-03").unwrap()), 3);
        assert_eq!(start.span_days(Date::parse("2026-09-30").unwrap()), 0);
        let leap = Date::parse("2024-02-28").unwrap();
        assert_eq!(leap.span_days(Date::parse("2024-03-01").unwrap()), 3);
    }

    #[test]
    fn unix_seconds_render_as_utc() {
        assert_eq!(format_utc_date(1_716_359_728), "2024-05-22");
        assert_eq!(format_utc_datetime(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_utc_datetime(1_716_359_728), "2024-05-22T06:35:28Z");
    }
}
