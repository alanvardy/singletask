use crate::error::Error;
use crate::request;
use chrono::DateTime;
use chrono::NaiveDate;
use chrono_tz::Tz;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt::Display;
use tokio::task::JoinHandle;
use urlencoding::encode;
use uuid::Uuid;

const SYNC_URL: &str = "/sync/v9/sync";
const REST_V2_TASKS_URL: &str = "/rest/v2/tasks/";

// Completes task inside another thread
pub fn spawn_complete_task(
    token: &str,
    task_id: &str,
    test_server: &Option<String>,
) -> JoinHandle<Result<String, Error>> {
    let token = token.to_owned();
    let task_id = task_id.to_owned();
    let test_server = test_server.clone();
    tokio::spawn(async move { complete_task(&token, &task_id, &test_server).await })
}

/// Complete the last task returned by "next task"
pub async fn complete_task(
    token: &str,
    task_id: &str,
    test_server_url: &Option<String>,
) -> Result<String, Error> {
    let uuid = Uuid::new_v4().to_string();

    let body = json!({"commands": [{"type": "item_close", "uuid": uuid, "temp_id": uuid, "args": {"id": task_id}}]});
    let url = String::from(SYNC_URL);

    request::post_todoist_sync(token, &url, body, test_server_url).await?;

    // Does not pass back a task
    Ok(String::from("✓"))
}
pub async fn all_tasks(
    token: &str,
    filter: &str,
    test_server_url: &Option<String>,
) -> Result<Vec<Task>, Error> {
    let filters = filter.split(",").collect::<Vec<&str>>();

    let mut handles = Vec::new();
    for f in filters {
        handles.push(tasks_for_filter(token, f, test_server_url.clone()));
    }

    let mut tasks = Vec::new();
    for list_of_tasks in join_all(handles).await {
        tasks.extend(list_of_tasks?);
    }

    Ok(tasks)
}

pub async fn tasks_for_filter(
    token: &str,
    filter: &str,
    test_server_url: Option<String>,
) -> Result<Vec<Task>, Error> {
    let encoded = encode(filter);
    let url = format!("{REST_V2_TASKS_URL}?filter={encoded}");
    let json = request::get_todoist_rest(token, &url, test_server_url).await?;
    rest_json_to_tasks(json)
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
        write!(f, "{text}")
    }
}

pub fn rest_json_to_tasks(json: String) -> Result<Vec<Task>, Error> {
    let tasks: Vec<Task> = serde_json::from_str(&json)?;
    Ok(tasks)
}

#[allow(dead_code)]
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
