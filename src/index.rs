use std::collections::HashMap;

use crate::tasks;
use crate::tasks::Task;
use crate::Link;
use askama_axum::Template;
use axum::{
    extract::{Path, Query},
    response::Html,
    routing::get,
    Router,
};

const FILTER: &str = "tod | overdue";
const TIMEZONE: &str = "America/Los Angeles";

pub fn routes() -> Router {
    Router::new()
        .route("/", get(index))
        .route("/:token", get(index_with_token))
        .route("/:token/complete/:task_id", get(complete_task))
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    title: String,
    navigation: Vec<Link>,
}

async fn index(Query(params): Query<HashMap<String, String>>) -> Html<String> {
    dbg!(params.clone());

    let has_token = params.contains_key("token");
    let has_filter = params.contains_key("filter");
    let has_timezone = params.contains_key("timezone");
    let has_task_id = params.contains_key("task_id");

    if !has_task_id && has_token && has_filter && has_timezone {
        let filter = params.get("filter").unwrap();
        let timezone = params.get("timezone").unwrap();
        let token = params.get("token").unwrap();

        let tasks = tasks::all_tasks(token, filter, timezone).await;
        if let Some(task) = tasks.unwrap().first() {
            let index = IndexWithTokenTemplate {
                title: "SingleTask".into(),
                navigation: crate::get_nav(),
                token: token.to_owned(),
                filter: filter.to_owned(),
                timezone: timezone.to_owned(),
                task: task.clone(),
            };
            Html(index.render().unwrap())
        } else {
            let index = IndexNoTask {
                title: "SingleTask".into(),
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

        tasks::complete_task(token, task_id).await.unwrap();
        let tasks = tasks::all_tasks(token, filter, timezone).await;

        if let Some(task) = tasks.unwrap().first() {
            let index = IndexWithTokenTemplate {
                title: "SingleTask".into(),
                navigation: crate::get_nav(),
                token: token.to_owned(),
                filter: filter.to_owned(),
                timezone: timezone.to_owned(),
                task: task.clone(),
            };
            Html(index.render().unwrap())
        } else {
            let index = IndexNoTask {
                title: "SingleTask".into(),
                navigation: crate::get_nav(),
                token: token.to_owned(),
                filter: filter.to_owned(),
                timezone: timezone.to_owned(),
            };
            Html(index.render().unwrap())
        }
    } else {
        let index = IndexTemplate {
            title: "SingleTask".into(),
            navigation: crate::get_nav(),
        };

        Html(index.render().unwrap())
    }
}

#[derive(Template)]
#[template(path = "index_with_token.html")]
struct IndexWithTokenTemplate {
    title: String,
    navigation: Vec<Link>,
    token: String,
    timezone: String,
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

async fn index_with_token(Path(token): Path<String>) -> Html<String> {
    let filter = FILTER.to_string();
    let timezone = TIMEZONE.to_string();
    let tasks = tasks::all_tasks(&token, &filter, &timezone).await;
    let task = tasks.unwrap().first().unwrap().clone();

    let index = IndexWithTokenTemplate {
        title: "SingleTask".into(),
        navigation: crate::get_nav(),
        filter,
        timezone,
        token,
        task,
    };

    Html(index.render().unwrap())
}
async fn complete_task(Path((token, task_id)): Path<(String, String)>) -> Html<String> {
    tasks::complete_task(&token, &task_id).await.unwrap();
    let filter = FILTER.to_string();
    let timezone = TIMEZONE.to_string();
    let tasks = tasks::all_tasks(&token, &filter, &timezone).await;
    let task = tasks.unwrap().first().unwrap().clone();

    let index = IndexWithTokenTemplate {
        title: "SingleTask".into(),
        navigation: crate::get_nav(),
        token,
        filter,
        timezone,
        task,
    };

    Html(index.render().unwrap())
}
