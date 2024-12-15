use crate::error::{self, Error};
use chrono::offset::Utc;
use chrono::DateTime;
use chrono_tz::Tz;
// use regex::Regex;

pub fn now(timezone: &Tz) -> Result<DateTime<Tz>, Error> {
    Ok(Utc::now().with_timezone(timezone))
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

/// How far in the past a datetime is, in minutes.
/// Postive in past, negative in future.
pub fn age_in_minutes(datetime: DateTime<Tz>, timezone: &Tz) -> Result<i64, Error> {
    let num_minutes = -datetime.signed_duration_since(now(timezone)?).num_minutes();
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
