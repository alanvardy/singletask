use crate::error::Error;
use crate::tasks::Task;
use crate::tasks::{self, Priority};
use crate::unsplash;
use crate::unsplash::Unsplash;
use crate::user;
use crate::{time, AppState, Link, UserState};
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::{extract::Query, response::Html, routing::get, Router};
use chrono_tz::Tz;
use comrak::Options;
use std::collections::HashMap;
use std::sync::Arc;

const CACHE_TASKS_MAX_AGE_MINUTES: i64 = 15;

pub fn routes(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/process", get(process))
        .with_state(app_state)
}

#[derive(Template)]
#[template(path = "process_with_task.html")]
struct ProcessWithTask {
    title: String,
    navigation: Vec<Link>,
    token: String,
    content_color_class: String,
    task: Task,
    filter: String,
    unsplash: Unsplash,
}

#[derive(Template)]
#[template(path = "process_with_no_task.html")]
struct ProcessNoTask {
    title: String,
    navigation: Vec<Link>,
    token: String,
    filter: String,
    unsplash: Unsplash,
}

async fn process(
    State(app_state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Html<String>, Error> {
    let has_complete_task_id = params.contains_key("complete_task_id");
    let skip_task_id = params.get("skip_task_id");
    let filter = fetch_parameter(&params, "filter")?;
    let token = fetch_parameter(&params, "token")?;
    let key = format!("{token}{filter}");
    let test_server_url = &app_state.clone().test_server_url;
    let user_state = get_or_create_user_state(app_state.clone(), &key).await?;
    let timezone =
        user::cached_get_timezone(&app_state, &user_state, &token, &key, test_server_url).await?;
    let unsplash = unsplash::cached_get_random(&app_state, &user_state, &timezone, key).await?;
    let mut title = filter.clone();
    title.truncate(20);

    if !has_complete_task_id {
        let tasks = get_tasks(
            app_state,
            &token,
            &filter,
            &timezone,
            None,
            skip_task_id,
            test_server_url,
        )
        .await;
        if let Some(task) = tasks?.first() {
            let index = ProcessWithTask {
                title,
                navigation: crate::get_nav(),
                token: token.to_owned(),
                filter: filter.to_owned(),
                content_color_class: get_content_color_class(task),
                task: task.clone(),
                unsplash,
            };
            Ok(Html(index.render()?))
        } else {
            let index = ProcessNoTask {
                title,
                navigation: crate::get_nav(),
                token: token.to_owned(),
                filter: filter.to_owned(),
                unsplash,
            };
            Ok(Html(index.render()?))
        }
    } else {
        let complete_task_id = fetch_parameter(&params, "complete_task_id")?;

        let handle = tasks::spawn_complete_task(&token, &complete_task_id, test_server_url);
        let tasks = get_tasks(
            app_state,
            &token,
            &filter,
            &timezone,
            Some(&complete_task_id),
            skip_task_id,
            test_server_url,
        )
        .await;
        let _ = handle.await?;

        if let Some(task) = tasks?.first() {
            let index = ProcessWithTask {
                title,
                navigation: crate::get_nav(),
                token: token.to_owned(),
                filter: filter.to_owned(),
                content_color_class: get_content_color_class(task),
                task: task.clone(),
                unsplash,
            };
            Ok(Html(index.render()?))
        } else {
            let index = ProcessNoTask {
                title,
                navigation: crate::get_nav(),
                token: token.to_owned(),
                filter: filter.to_owned(),
                unsplash,
            };
            Ok(Html(index.render()?))
        }
    }
}

fn fetch_parameter(params: &HashMap<String, String>, field: &str) -> Result<String, Error> {
    params
        .get(field)
        .ok_or_else(|| Error {
            code: StatusCode::BAD_REQUEST,
            message: format!("Missing query parameter: {field}"),
            source: "fetch_parameter".to_string(),
        })
        .cloned()
}

fn get_content_color_class(task: &Task) -> String {
    match task.priority {
        Priority::None => String::from("has-text-white"),
        Priority::Low => String::from("has-text-primary"),
        Priority::Medium => String::from("has-text-warning"),
        Priority::High => String::from("has-text-danger"),
    }
}

async fn get_or_create_user_state(app_state: Arc<AppState>, key: &str) -> Result<UserState, Error> {
    let db = &app_state.clone().db;
    let maybe_user_state = db.begin(false).await.get(key.to_string())?;

    if let Some(user_state) = maybe_user_state {
        Ok(user_state)
    } else {
        Ok(UserState {
            tasks: Vec::new(),
            skip_task_ids: Vec::new(),
            tasks_updated_at: None,
            unsplash: None,
            unsplash_updated_at: None,
            timezone: None,
        })
    }
}

async fn get_tasks(
    app_state: Arc<AppState>,
    token: &str,
    filter: &str,
    timezone: &Tz,
    complete_task_id: Option<&str>,
    skip_task_id: Option<&String>,
    test_server_url: &Option<String>,
) -> Result<Vec<Task>, Error> {
    let key = format!("{token}{filter}");

    let user_state = get_or_create_user_state(app_state.clone(), &key).await?;
    let skip_task_ids = if let Some(task_id) = skip_task_id {
        vec![task_id.to_string()]
    } else {
        Vec::new()
    };

    let db = &app_state.clone().db;
    if has_cached_tasks(&user_state, timezone, complete_task_id, &skip_task_ids)? {
        println!("CACHE HIT");
        let skip_task_ids = merge_skip_task_ids(&user_state, skip_task_id);
        let tasks =
            filter_completed_task(user_state.tasks.clone(), complete_task_id, &skip_task_ids);
        let mut tx = db.begin(true).await;
        let user_state = UserState {
            tasks: tasks.clone(),
            skip_task_ids,
            ..user_state
        };
        tx.set(key.clone(), user_state)?;
        tx.commit()?;

        Ok(markdown_to_html(tasks))
    } else {
        println!("CACHE EXPIRED OR NO TASKS");
        let tasks = tasks::all_tasks(token, filter, test_server_url).await?;
        let tasks = filter_completed_task(tasks, complete_task_id, &skip_task_ids);
        let mut tx = db.begin(true).await;
        let tasks_updated_at = time::now(timezone)?;
        let user_state = UserState {
            tasks: tasks.clone(),
            skip_task_ids,
            tasks_updated_at: Some(tasks_updated_at),
            ..user_state.clone()
        };
        tx.set(key.clone(), user_state)?;
        tx.commit()?;

        Ok(markdown_to_html(tasks))
    }
}

fn markdown_to_html(tasks: Vec<Task>) -> Vec<Task> {
    let options = Options::default();
    tasks
        .into_iter()
        .map(|t| Task {
            content: comrak::markdown_to_html(&t.content, &options),
            description: comrak::markdown_to_html(&t.description, &options),
            ..t
        })
        .collect()
    //     use comrak::{markdown_to_html, Options};
    // assert_eq!(markdown_to_html("Hello, **世界**!", &Options::default()),
    //            "<p>Hello, <strong>世界</strong>!</p>\n");
}

fn merge_skip_task_ids(user_state: &UserState, skip_task_id: Option<&String>) -> Vec<String> {
    if let Some(skip_task_id) = skip_task_id {
        let mut skip_task_ids = user_state.skip_task_ids.clone();
        skip_task_ids.push(skip_task_id.to_string());
        skip_task_ids
    } else {
        user_state.skip_task_ids.clone()
    }
}

fn has_cached_tasks(
    user_state: &UserState,
    timezone: &Tz,
    complete_task_id: Option<&str>,
    skip_task_ids: &[String],
) -> Result<bool, Error> {
    if let Some(updated_at) = user_state.tasks_updated_at {
        let age = time::age_in_minutes(updated_at, timezone)?;
        let more_tasks = more_tasks(user_state, complete_task_id, skip_task_ids);

        if age < CACHE_TASKS_MAX_AGE_MINUTES && more_tasks {
            return Ok(true);
        }
    }
    Ok(false)
}

fn filter_completed_task(
    tasks: Vec<Task>,
    complete_task_id: Option<&str>,
    skip_task_ids: &[String],
) -> Vec<Task> {
    tasks
        .into_iter()
        .filter(|t| t.id != complete_task_id.unwrap_or_default() && !skip_task_ids.contains(&t.id))
        .collect::<Vec<Task>>()
}

/// Checks if there are more tasks to process (beyond the one that we are now completing)
fn more_tasks(state: &UserState, complete_task_id: Option<&str>, skip_task_ids: &[String]) -> bool {
    let tasks = filter_completed_task(state.tasks.clone(), complete_task_id, skip_task_ids);
    !tasks.is_empty()
}
