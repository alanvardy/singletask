use crate::error::Error;
use crate::tasks::Task;
use crate::tasks::{self, Priority};
use crate::unsplash;
use crate::unsplash::Unsplash;
use crate::{time, AppState, Link, UserState};
use askama_axum::Template;
use axum::extract::State;
use axum::{extract::Query, response::Html, routing::get, Router};
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
    timezone: String,
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
    timezone: String,
    filter: String,
    unsplash: Unsplash,
}

async fn process(
    State(app_state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Html<String> {
    let has_complete_task_id = params.contains_key("complete_task_id");
    let skip_task_id = params.get("skip_task_id");
    let filter = params.get("filter").unwrap();
    let timezone = params.get("timezone").unwrap();
    let token = params.get("token").unwrap();
    let key = format!("{token}{filter}");

    let user_state = get_or_create_user_state(app_state.clone(), &key, timezone)
        .await
        .unwrap();
    let unsplash = unsplash::cached_get_random(&app_state, &user_state, timezone, key)
        .await
        .unwrap();
    let mut title = filter.clone();
    title.truncate(20);

    if !has_complete_task_id {
        let tasks = get_tasks(app_state, token, filter, timezone, None, skip_task_id).await;
        if let Some(task) = tasks.unwrap().first() {
            let index = ProcessWithTask {
                title,
                navigation: crate::get_nav(),
                token: token.to_owned(),
                filter: filter.to_owned(),
                content_color_class: get_content_color_class(task),
                timezone: timezone.to_owned(),
                task: task.clone(),
                unsplash,
            };
            Html(index.render().unwrap())
        } else {
            let index = ProcessNoTask {
                title,
                navigation: crate::get_nav(),
                token: token.to_owned(),
                filter: filter.to_owned(),
                timezone: timezone.to_owned(),
                unsplash,
            };
            Html(index.render().unwrap())
        }
    } else {
        let complete_task_id = params.get("complete_task_id").unwrap();

        let handle = tasks::spawn_complete_task(token, complete_task_id);
        let tasks = get_tasks(
            app_state,
            token,
            filter,
            timezone,
            Some(complete_task_id),
            skip_task_id,
        )
        .await;
        let _ = handle.await.unwrap();

        if let Some(task) = tasks.unwrap().first() {
            let index = ProcessWithTask {
                title,
                navigation: crate::get_nav(),
                token: token.to_owned(),
                filter: filter.to_owned(),
                timezone: timezone.to_owned(),
                content_color_class: get_content_color_class(task),
                task: task.clone(),
                unsplash,
            };
            Html(index.render().unwrap())
        } else {
            let index = ProcessNoTask {
                title,
                navigation: crate::get_nav(),
                token: token.to_owned(),
                filter: filter.to_owned(),
                timezone: timezone.to_owned(),
                unsplash,
            };
            Html(index.render().unwrap())
        }
    }
}

fn get_content_color_class(task: &Task) -> String {
    match task.priority {
        Priority::None => String::from("has-text-white"),
        Priority::Low => String::from("has-text-primary"),
        Priority::Medium => String::from("has-text-warning"),
        Priority::High => String::from("has-text-danger"),
    }
}

async fn get_or_create_user_state(
    app_state: Arc<AppState>,
    key: &str,
    timezone: &str,
) -> Result<UserState, Error> {
    let db = &app_state.clone().db;
    let maybe_user_state = db.begin(false).await?.get(key.to_string())?;

    if let Some(user_state) = maybe_user_state {
        Ok(user_state)
    } else {
        Ok(UserState {
            tasks: Vec::new(),
            skip_task_ids: Vec::new(),
            tasks_updated_at: time::now(timezone)?,
            unsplash: None,
            unsplash_updated_at: time::now(timezone)?,
        })
    }
}

async fn get_tasks(
    app_state: Arc<AppState>,
    token: &str,
    filter: &str,
    timezone: &str,
    complete_task_id: Option<&str>,
    skip_task_id: Option<&String>,
) -> Result<Vec<Task>, Error> {
    let key = format!("{token}{filter}");

    let user_state = get_or_create_user_state(app_state.clone(), &key, timezone).await?;
    let skip_task_ids = if let Some(task_id) = skip_task_id {
        vec![task_id.to_string()]
    } else {
        Vec::new()
    };

    let db = &app_state.clone().db;
    match determine_freshness(user_state, timezone, complete_task_id, &skip_task_ids)? {
        CacheResult::Hit(user_state) => {
            println!("CACHE HIT");
            let skip_task_ids = merge_skip_task_ids(&user_state, skip_task_id);
            let tasks =
                filter_completed_task(user_state.tasks.clone(), complete_task_id, &skip_task_ids);
            let mut tx = db.begin(true).await?;
            let user_state = UserState {
                tasks: tasks.clone(),
                skip_task_ids,
                ..user_state
            };
            tx.set(key.clone(), user_state)?;
            tx.commit()?;

            Ok(tasks)
        }
        CacheResult::Expired(user_state) => {
            println!("CACHE EXPIRED OR NO TASKS");
            let tasks = tasks::all_tasks(token, filter, timezone).await?;
            let tasks = filter_completed_task(tasks, complete_task_id, &skip_task_ids);
            let mut tx = db.begin(true).await?;
            let tasks_updated_at = time::now(timezone)?;
            let user_state = UserState {
                tasks: tasks.clone(),
                skip_task_ids,
                tasks_updated_at,
                ..user_state.clone()
            };
            tx.set(key.clone(), user_state)?;
            tx.commit()?;

            Ok(tasks)
        }
    }
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

fn determine_freshness(
    user_state: UserState,
    timezone: &str,
    complete_task_id: Option<&str>,
    skip_task_ids: &[String],
) -> Result<CacheResult, Error> {
    if time::age_in_minutes(user_state.tasks_updated_at, timezone)? < CACHE_TASKS_MAX_AGE_MINUTES
        && more_tasks(&user_state, complete_task_id, skip_task_ids)
    {
        Ok(CacheResult::Hit(user_state))
    } else {
        Ok(CacheResult::Expired(user_state))
    }
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

enum CacheResult {
    /// We have data but it is old or there are no tasks remaining
    Expired(UserState),
    // We have recent data
    Hit(UserState),
}
