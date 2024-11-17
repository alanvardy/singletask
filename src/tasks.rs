use std::fmt::Display;

use crate::error;
use crate::error::Error;
use crate::time;
use chrono::offset::Utc;
use chrono::DateTime;
use chrono::NaiveDate;
use chrono::NaiveDateTime;
use chrono_tz::Tz;
use reqwest::Client;
use reqwest::Response;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use reqwest::header::AUTHORIZATION;
use reqwest::header::CONTENT_TYPE;

const SYNC_URL: &str = "/sync/v9/sync";
const REST_V2_TASKS_URL: &str = "/rest/v2/tasks/";
const TODOIST_URL: &str = "https://api.todoist.com";

/// Complete the last task returned by "next task"
pub async fn complete_task(token: &String, task_id: &String) -> Result<String, Error> {
    let uuid = Uuid::new_v4().to_string();

    let body = json!({"commands": [{"type": "item_close", "uuid": uuid, "temp_id": uuid, "args": {"id": task_id}}]});
    let url = String::from(SYNC_URL);

    post_todoist_sync(token, &url, body).await?;

    // Does not pass back a task
    Ok(String::from("✓"))
}
/// Post to Todoist via sync API
/// We use sync when we want natural languague processing.
pub async fn post_todoist_sync(
    token: &String,
    url: &String,
    body: serde_json::Value,
) -> Result<String, Error> {
    let request_url = format!("{TODOIST_URL}{url}");

    let response = Client::new()
        .post(request_url.clone())
        .header(CONTENT_TYPE, "application/json")
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .json(&body)
        .send()
        .await?;

    handle_response(response, "POST", url, body).await
}

pub async fn all_tasks(
    token: &String,
    filter: &String,
    timezone: &String,
) -> Result<Vec<Task>, Error> {
    let tasks = tasks_for_filter(token, filter).await?;

    Ok(sort_by_datetime(tasks, timezone))
}

pub async fn tasks_for_filter(token: &String, filter: &String) -> Result<Vec<Task>, Error> {
    use urlencoding::encode;

    let encoded = encode(filter);
    let url = format!("{REST_V2_TASKS_URL}?filter={encoded}");
    let json = get_todoist_rest(token, &url).await?;
    rest_json_to_tasks(json)
}

pub fn sort_by_datetime(mut tasks: Vec<Task>, timezone: &String) -> Vec<Task> {
    tasks.sort_by_key(|i| i.datetime(timezone));
    tasks
}
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Duration {
    pub amount: u32,
    pub unit: Unit,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum Unit {
    #[serde(rename(deserialize = "minute"))]
    Minute,
    #[serde(rename(deserialize = "day"))]
    Day,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Task {
    pub id: String,
    pub content: String,
    pub priority: Priority,
    pub description: String,
    pub labels: Vec<String>,
    pub parent_id: Option<String>,
    pub project_id: String,
    pub due: Option<DateInfo>,
    /// Only on rest api return value
    pub is_completed: Option<bool>,
    pub is_deleted: Option<bool>,
    /// only on sync api return value
    pub checked: Option<bool>,
    pub duration: Option<Duration>,
}
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct DateInfo {
    pub date: String,
    pub is_recurring: bool,
    pub string: String,
    pub timezone: Option<String>,
}

#[derive(serde_repr::Serialize_repr, serde_repr::Deserialize_repr, Debug, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum Priority {
    None = 1,
    Low = 2,
    Medium = 3,
    High = 4,
}
impl Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Priority::None => "None",
            Priority::Low => "Low",
            Priority::Medium => "Medium",
            Priority::High => "High",
        };
        write!(f, "{}", text)
    }
}

impl Task {
    /// Return the value of the due field
    fn datetime(&self, timezone: &String) -> Option<DateTime<Tz>> {
        match self.datetimeinfo(timezone) {
            Ok(DateTimeInfo::DateTime { datetime, .. }) => Some(datetime),
            Ok(DateTimeInfo::Date { date, .. }) => {
                let naive_datetime = date.and_hms_opt(23, 59, 00)?;

                let now = time::now(timezone).ok()?;

                Some(DateTime::from_naive_utc_and_offset(
                    naive_datetime,
                    *now.offset(),
                ))
            }
            Ok(DateTimeInfo::NoDateTime) => None,
            Err(_) => None,
        }
    }
    /// Converts the JSON date representation into Date or Datetime
    fn datetimeinfo(&self, timezone: &String) -> Result<DateTimeInfo, Error> {
        let tz = match self.clone().due {
            None => time::timezone_from_str(timezone)?,
            Some(DateInfo { timezone: None, .. }) => time::timezone_from_str(timezone)?,
            Some(DateInfo {
                timezone: Some(tz_string),
                ..
            }) => time::timezone_from_str(&tz_string)?,
        };
        match self.clone().due {
            None => Ok(DateTimeInfo::NoDateTime),
            Some(DateInfo {
                date,
                is_recurring,
                string,
                ..
            }) if date.len() == 10 => Ok(DateTimeInfo::Date {
                date: time::date_from_str(&date, tz)?,
                is_recurring,
                string,
            }),
            Some(DateInfo {
                date,
                is_recurring,
                string,
                ..
            }) => Ok(DateTimeInfo::DateTime {
                datetime: time::datetime_from_str(&date, tz)?,
                is_recurring,
                string,
            }),
        }
    }
}
pub fn rest_json_to_tasks(json: String) -> Result<Vec<Task>, Error> {
    let tasks: Vec<Task> = serde_json::from_str(&json)?;
    Ok(tasks)
}

// Combine get and post into one function
/// Get Todoist via REST api
pub async fn get_todoist_rest(token: &String, url: &String) -> Result<String, Error> {
    let request_url = format!("{TODOIST_URL}{url}");
    let authorization: &str = &format!("Bearer {token}");
    let response = Client::new()
        .get(request_url.clone())
        .header(CONTENT_TYPE, "application/json")
        .header(AUTHORIZATION, authorization)
        .send()
        .await?;

    handle_response(response, "GET", url, json!({})).await
}

enum DateTimeInfo {
    NoDateTime,
    Date {
        date: NaiveDate,
        is_recurring: bool,
        string: String,
    },
    DateTime {
        datetime: DateTime<Tz>,
        is_recurring: bool,
        string: String,
    },
}

async fn handle_response(
    response: Response,
    method: &str,
    url: &String,
    body: serde_json::Value,
) -> Result<String, Error> {
    if response.status().is_success() {
        Ok(response.text().await?)
    } else {
        let json_string = response.text().await?;
        Err(error::new(
            "reqwest",
            &format!(
                "
            method: {method}
            url: {url}
            body: {body}
            response: {json_string}",
            ),
        ))
    }
}
