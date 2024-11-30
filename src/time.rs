use crate::error::{self, Error};
use chrono::offset::Utc;
use chrono::{DateTime, NaiveDate, NaiveDateTime};
use chrono_tz::Tz;
// use regex::Regex;

pub fn now(timezone: &str) -> Result<DateTime<Tz>, Error> {
    let tz = timezone_from_str(timezone)?;
    Ok(Utc::now().with_timezone(&tz))
}

// Return today's date in format 2021-09-16
// pub fn today_string(timezone: &String) -> Result<String, Error> {
//     Ok(now(timezone)?.format("%Y-%m-%d").to_string())
// }

// Return today's date in Utc
// pub fn today_date(timezone: &String) -> Result<NaiveDate, Error> {
//     Ok(now(timezone)?.date_naive())
// }

// pub fn datetime_is_today(datetime: DateTime<Tz>, timezone: String) -> Result<bool, Error> {
//     date_is_today(datetime.date_naive(), timezone)
// }

// pub fn date_is_today(date: NaiveDate, timezone: String) -> Result<bool, Error> {
//     let date_string = date.format("%Y-%m-%d").to_string();
//     let today_string = today_string(&timezone)?;
//     Ok(date_string == today_string)
// }

// pub fn date_is_today(date: NaiveDate, timezone: String) -> Result<bool, Error> {
//     let date_string = date.format("%Y-%m-%d").to_string();
//     let today_string = today_string(&timezone)?;
//     Ok(date_string == today_string)
// }

/// How far in the past a datetime is, in minutes
pub fn age_in_minutes(datetime: DateTime<Tz>, timezone: &str) -> Result<i64, Error> {
    let num_minutes = datetime.signed_duration_since(now(timezone)?).num_minutes();
    Ok(num_minutes)
}

// pub fn format_date(date: &NaiveDate, timezone: String) -> Result<String, Error> {
//     if date_is_today(*date, timezone)? {
//         Ok(String::from("Today"))
//     } else {
//         Ok(date.format("%Y-%m-%d").to_string())
//     }
// }

// pub fn format_datetime(datetime: &DateTime<Tz>, timezone: String) -> Result<String, Error> {
//     let tz = timezone_from_str(&timezone)?;
//     if datetime_is_today(*datetime, timezone)? {
//         Ok(datetime.with_timezone(&tz).format("%H:%M").to_string())
//     } else {
//         Ok(datetime.with_timezone(&tz).to_string())
//     }
// }

/// Parse DateTime
pub fn datetime_from_str(str: &str, timezone: Tz) -> Result<DateTime<Tz>, Error> {
    match str.len() {
        19 => parse_datetime_from_19(str, timezone),
        20 => parse_datetime_from_20(str),
        _ => Err(error::new(
            "datetime_from_str",
            "cannot parse DateTime: {str}",
        )),
    }
}

pub fn parse_datetime_from_19(str: &str, timezone: Tz) -> Result<DateTime<Tz>, Error> {
    let tz = NaiveDateTime::parse_from_str(str, "%Y-%m-%dT%H:%M:%S")?
        .and_local_timezone(timezone)
        .unwrap();
    Ok(tz)
}

pub fn parse_datetime_from_20(str: &str) -> Result<DateTime<Tz>, Error> {
    let tz = NaiveDateTime::parse_from_str(str, "%Y-%m-%dT%H:%M:%SZ")?
        .and_local_timezone(Tz::UTC)
        .unwrap();
    Ok(tz)
}

pub fn timezone_from_str(timezone: &str) -> Result<Tz, Error> {
    match timezone.parse::<Tz>() {
        Ok(tz) => Ok(tz),
        Err(_) => parse_gmt_to_timezone(timezone),
    }
}

/// For when we get offsets like GMT -7:00
fn parse_gmt_to_timezone(gmt: &str) -> Result<Tz, Error> {
    let split: Vec<&str> = gmt.split_whitespace().collect();
    let offset = split
        .get(1)
        .ok_or_else(|| error::new("parse_timezone", "Could not get offset"))?;
    let offset = offset.replace(":00", "");
    let offset = offset.replace(':', "");
    let offset_num = offset.parse::<i32>()?;

    let tz_string = format!(
        "Etc/GMT{}",
        if offset_num < 0 {
            "+".to_string()
        } else {
            "-".to_string()
        } + &offset_num.abs().to_string()
    );
    tz_string.parse().map_err(Error::from)
}

/// Parse Date
pub fn date_from_str(str: &str, timezone: Tz) -> Result<NaiveDate, Error> {
    let date = match str.len() {
        10 => NaiveDate::parse_from_str(str, "%Y-%m-%d")?,
        19 => NaiveDateTime::parse_from_str(str, "%Y-%m-%dT%H:%M:%S")?
            .and_local_timezone(timezone)
            .unwrap()
            .date_naive(),
        20 => NaiveDateTime::parse_from_str(str, "%Y-%m-%dT%H:%M:%SZ")?
            .and_local_timezone(timezone)
            .unwrap()
            .date_naive(),
        _ => {
            return Err(error::new(
                "date_from_str",
                "cannot parse NaiveDate, unknown length: {str}",
            ))
        }
    };

    Ok(date)
}

// Checks if string is a date in format YYYY-MM-DD
// pub fn is_date(string: &str) -> bool {
//     let re = Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
//     re.is_match(string)
// }

// Checks if string is a datetime in format YYYY-MM-DD HH:MM
// pub fn is_datetime(string: &str) -> bool {
//     let re = Regex::new(r"^\d{4}-\d{2}-\d{2} \d{2}:\d{2}$").unwrap();
//     re.is_match(string)
// }

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn test_is_date() {
    //     assert!(is_date("2022-10-05"));
    //     assert!(!is_date("22-10-05"));
    //     assert!(!is_date("2022-10-05 24:02"));
    //     assert!(!is_date("today"));
    // }

    // #[test]
    // fn test_is_datetime() {
    //     assert!(!is_datetime("2022-10-05"));
    //     assert!(!is_datetime("22-10-05"));
    //     assert!(is_datetime("2022-10-05 24:02"));
    //     assert!(!is_datetime("today"));
    // }

    #[test]
    fn test_timezone_from_string() {
        assert_eq!(
            timezone_from_str("America/Los_Angeles"),
            Ok(Tz::America__Los_Angeles),
        );

        assert_eq!(timezone_from_str("GMT -7:00"), Ok(Tz::Etc__GMTPlus7),);
    }
}
