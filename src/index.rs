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
    let has_task_id = params.contains_key("task_id");

    if !has_task_id && has_token && has_filter && has_timezone {
        let filter = params.get("filter").unwrap();
        let timezone = params.get("timezone").unwrap();
        let token = params.get("token").unwrap();
        let mut title = filter.clone();
        title.truncate(20);

        let tasks = get_tasks(state, token, filter, timezone, None).await;
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
    } else if has_task_id && has_token && has_filter && has_timezone {
        let task_id = params.get("task_id").unwrap();
        let token = params.get("token").unwrap();
        let filter = params.get("filter").unwrap();
        let timezone = params.get("timezone").unwrap();
        let mut title = filter.clone();
        title.truncate(20);

        let handle = tasks::spawn_complete_task(token, task_id);
        let tasks = get_tasks(state, token, filter, timezone, Some(task_id)).await;
        let _ = handle.await.unwrap();

        if let Some(task) = tasks
            .unwrap()
            .into_iter()
            .filter(|t| *t.id != *task_id)
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
    task_id: Option<&str>,
) -> Result<Vec<Task>, Error> {
    let key = format!("{token}{filter}");

    let db = &state.clone().db;
    let maybe_user_state = db.begin(false).await?.get(key.clone())?;

    match determine_freshness(maybe_user_state, timezone, task_id)? {
        Action::Fresh(user_state) => {
            let tasks = filter_completed_task(user_state.tasks, task_id);
            let mut tx = db.begin(true).await?;
            let user_state = UserState {
                tasks: tasks.clone(),
                ..user_state
            };
            tx.set(key.clone(), user_state)?;
            tx.commit()?;

            Ok(tasks)
        }
        Action::Expired(_user_state) => {
            let tasks = tasks::all_tasks(token, filter, timezone).await?;
            let tasks = filter_completed_task(tasks, task_id);
            let mut tx = db.begin(true).await?;
            let updated_at = time::now(timezone)?;
            let user_state = UserState {
                tasks: tasks.clone(),
                updated_at,
            };
            tx.set(key.clone(), user_state)?;
            tx.commit()?;

            Ok(tasks)
        }
        Action::Missing => {
            let tasks = tasks::all_tasks(token, filter, timezone).await?;

            let tasks = filter_completed_task(tasks, task_id);
            let mut tx = db.begin(true).await?;
            let updated_at = time::now(timezone)?;
            let user_state = UserState {
                tasks: tasks.clone(),
                updated_at,
            };
            tx.set(key.clone(), user_state)?;
            tx.commit()?;

            Ok(tasks)
        }
    }
}

fn determine_freshness(
    user_state: Option<UserState>,
    timezone: &str,
    task_id: Option<&str>,
) -> Result<Action, Error> {
    if let Some(state) = user_state {
        if time::age_in_minutes(state.updated_at, timezone)? < CACHE_TASKS_MAX_AGE_MINUTES
            && more_tasks(&state, task_id)
        {
            Ok(Action::Fresh(state))
        } else {
            Ok(Action::Expired(state))
        }
    } else {
        Ok(Action::Missing)
    }
}

fn filter_completed_task(tasks: Vec<Task>, task_id: Option<&str>) -> Vec<Task> {
    tasks
        .into_iter()
        .filter(|t| t.id != task_id.unwrap_or_default())
        .collect::<Vec<Task>>()
}

/// Checks if there are more tasks to process (beyond the one that we are now completing)
fn more_tasks(state: &UserState, task_id: Option<&str>) -> bool {
    let tasks = filter_completed_task(state.tasks.clone(), task_id);
    !tasks.is_empty()
}

enum Action {
    /// We have data but it is old or there are no tasks remaining
    Expired(UserState),
    // We have recent data
    Fresh(UserState),
    /// There is no data
    Missing,
}
