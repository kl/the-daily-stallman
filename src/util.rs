use anyhow::Result as AnyResult;
use anyhow::*;
use chrono::{DateTime, NaiveDate, NaiveDateTime};
use regex::Regex;

pub fn is_http(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

/// Accepts dates in RFC 3339 (e.g. 2019-01-01T00:00:00Z), without hh:mm:Z (e.g. 2019-01-01T12),
/// without time (e.g. 2019-01-01), and without day (e.g. 2019-01).
/// All formats are parsed as UTC (except RFC 3339 which can specify a time zone).
/// Missing date components are set to 1 and missing time components are set 0.
pub fn parse_date_time(input: &str) -> AnyResult<NaiveDateTime> {
    if let Ok(date) = DateTime::parse_from_rfc3339(input) {
        Ok(date.naive_utc())
    } else {
        let re = Regex::new(r"(\d{4})\D?(\d{2})\D?(\d{2})?\D?(\d{2})?").unwrap();
        let cap = re
            .captures(input)
            .ok_or_else(|| anyhow!("invalid date format"))?;

        let year: u32 = cap.get(1).unwrap().as_str().parse().unwrap();
        let month: u32 = cap.get(2).unwrap().as_str().parse().unwrap();
        let day: u32 = cap.get(3).map_or(1, |v| v.as_str().parse().unwrap());
        let hour: u32 = cap.get(4).map_or(0, |v| v.as_str().parse().unwrap());

        let date_err = || anyhow!("invalid or out-of-range date");
        let date_time = NaiveDate::from_ymd_opt(year as i32, month, day)
            .ok_or_else(date_err)?
            .and_hms_opt(hour, 0, 0)
            .ok_or_else(date_err)?;

        Ok(date_time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_date_time() {
        assert_eq!(
            parse_date_time("2019-01-01T00:00:00Z").unwrap(),
            date("2019-01-01T00:00:00Z")
        );
        assert_eq!(
            parse_date_time("2019-01-01T12").unwrap(),
            date("2019-01-01T12:00:00Z")
        );
        assert_eq!(
            parse_date_time("2019-01-15").unwrap(),
            date("2019-01-15T00:00:00Z")
        );
        assert_eq!(
            parse_date_time("2019-10").unwrap(),
            date("2019-10-01T00:00:00Z")
        );
        assert!(parse_date_time("2019-13-01").is_err());
        assert!(parse_date_time("2019-12-01T24:00:00").is_err());
    }

    fn date(d: &str) -> NaiveDateTime {
        chrono::DateTime::parse_from_rfc3339(d).unwrap().naive_utc()
    }
}
