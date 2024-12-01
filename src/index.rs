use crate::error::Error;
use crate::tasks::Task;
use crate::tasks::{self, Priority};
use crate::{time, AppState, Link, UserState};
use askama_axum::Template;
use axum::extract::State;
use axum::{extract::Query, response::Html, routing::get, Router};
use std::collections::HashMap;
use std::sync::Arc;

const CACHE_TASKS_MAX_AGE_MINUTES: i64 = 15;

pub fn routes() -> Router {
    let db = echodb::new::<String, UserState>();
    let shared_state = Arc::new(AppState { db });
    Router::new()
        .route("/", get(index))
        .with_state(shared_state)
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    title: String,
    navigation: Vec<Link>,
}

#[derive(Template)]
#[template(path = "index_with_task.html")]
struct IndexWithTask {
    title: String,
    navigation: Vec<Link>,
    token: String,
    timezone: String,
    content_color_class: String,
    task: Task,
    filter: String,
}

#[derive(Template)]
#[template(path = "index_with_no_task.html")]
struct IndexNoTask {
    title: String,
    navigation: Vec<Link>,
    token: String,
    timezone: String,
    filter: String,
}
async fn index(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Html<String> {
    let has_token = params.contains_key("token");
    let has_filter = params.contains_key("filter");
    let has_timezone = params.contains_key("timezone");
    let has_complete_task_id = params.contains_key("complete_task_id");
    let skip_task_id = params.get("skip_task_id");

    if !has_complete_task_id && has_token && has_filter && has_timezone {
        let filter = params.get("filter").unwrap();
        let timezone = params.get("timezone").unwrap();
        let token = params.get("token").unwrap();
        let mut title = filter.clone();
        title.truncate(20);

        let tasks = get_tasks(state, token, filter, timezone, None, skip_task_id).await;
        if let Some(task) = tasks.unwrap().first() {
            let index = IndexWithTask {
                title,
                navigation: crate::get_nav(),
                token: token.to_owned(),
                filter: filter.to_owned(),
                content_color_class: get_content_color_class(task),
                timezone: timezone.to_owned(),
                task: task.clone(),
            };
            Html(index.render().unwrap())
        } else {
            let index = IndexNoTask {
                title,
                navigation: crate::get_nav(),
                token: token.to_owned(),
                filter: filter.to_owned(),
                timezone: timezone.to_owned(),
            };
            Html(index.render().unwrap())
        }
    } else if has_complete_task_id && has_token && has_filter && has_timezone {
        let complete_task_id = params.get("complete_task_id").unwrap();
        let token = params.get("token").unwrap();
        let filter = params.get("filter").unwrap();
        let timezone = params.get("timezone").unwrap();
        let mut title = filter.clone();
        title.truncate(20);

        let handle = tasks::spawn_complete_task(token, complete_task_id);
        let tasks = get_tasks(
            state,
            token,
            filter,
            timezone,
            Some(complete_task_id),
            skip_task_id,
        )
        .await;
        let _ = handle.await.unwrap();

        if let Some(task) = tasks
            .unwrap()
            .into_iter()
            .filter(|t| *t.id != *complete_task_id)
            .collect::<Vec<Task>>()
            .first()
        {
            let index = IndexWithTask {
                title,
                navigation: crate::get_nav(),
                token: token.to_owned(),
                filter: filter.to_owned(),
                timezone: timezone.to_owned(),
                content_color_class: get_content_color_class(task),
                task: task.clone(),
            };
            Html(index.render().unwrap())
        } else {
            let index = IndexNoTask {
                title,
                navigation: crate::get_nav(),
                token: token.to_owned(),
                filter: filter.to_owned(),
                timezone: timezone.to_owned(),
            };
            Html(index.render().unwrap())
        }
    } else {
        let index = IndexTemplate {
            title: "Home".into(),
            navigation: crate::get_nav(),
        };

        Html(index.render().unwrap())
    }
}

fn get_content_color_class(task: &Task) -> String {
    match task.priority {
        Priority::None => String::new(),
        Priority::Low => String::from("has-text-primary"),
        Priority::Medium => String::from("has-text-warning"),
        Priority::High => String::from("has-text-danger"),
    }
}

async fn get_tasks(
    state: Arc<AppState>,
    token: &str,
    filter: &str,
    timezone: &str,
    complete_task_id: Option<&str>,
    skip_task_id: Option<&String>,
) -> Result<Vec<Task>, Error> {
    let key = format!("{token}{filter}");

    let db = &state.clone().db;
    let maybe_user_state = db.begin(false).await?.get(key.clone())?;

    let skip_task_ids = if let Some(task_id) = skip_task_id {
        vec![task_id.to_string()]
    } else {
        Vec::new()
    };

    match determine_freshness(maybe_user_state, timezone, complete_task_id, &skip_task_ids)? {
        CacheResult::Hit(user_state) => {
            println!("CACHE HIT");
            let skip_task_ids = merge_skip_task_ids(&user_state, skip_task_id);
            let tasks = filter_completed_task(user_state.tasks, complete_task_id, &skip_task_ids);
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
        CacheResult::Expired(_user_state) => {
            println!("CACHE EXPIRED OR NO TASKS");
            let tasks = tasks::all_tasks(token, filter, timezone).await?;
            let tasks = filter_completed_task(tasks, complete_task_id, &skip_task_ids);
            let mut tx = db.begin(true).await?;
            let updated_at = time::now(timezone)?;
            let user_state = UserState {
                tasks: tasks.clone(),
                skip_task_ids,
                updated_at,
            };
            tx.set(key.clone(), user_state)?;
            tx.commit()?;

            Ok(tasks)
        }
        CacheResult::Miss => {
            println!("CACHE MISS");
            let tasks = tasks::all_tasks(token, filter, timezone).await?;

            let tasks = filter_completed_task(tasks, complete_task_id, &skip_task_ids);
            let mut tx = db.begin(true).await?;
            let updated_at = time::now(timezone)?;
            let user_state = UserState {
                tasks: tasks.clone(),
                skip_task_ids,
                updated_at,
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
    user_state: Option<UserState>,
    timezone: &str,
    complete_task_id: Option<&str>,
    skip_task_ids: &[String],
) -> Result<CacheResult, Error> {
    if let Some(state) = user_state {
        if time::age_in_minutes(state.updated_at, timezone)? < CACHE_TASKS_MAX_AGE_MINUTES
            && more_tasks(&state, complete_task_id, skip_task_ids)
        {
            Ok(CacheResult::Hit(state))
        } else {
            Ok(CacheResult::Expired(state))
        }
    } else {
        Ok(CacheResult::Miss)
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
    /// There is no data
    Miss,
}
