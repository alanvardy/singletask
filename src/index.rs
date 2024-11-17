use std::collections::HashMap;

use crate::tasks::Task;
use crate::tasks::{self, Priority};
use crate::Link;
use askama_axum::Template;
use axum::{extract::Query, response::Html, routing::get, Router};

pub fn routes() -> Router {
    Router::new().route("/", get(index))
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
async fn index(Query(params): Query<HashMap<String, String>>) -> Html<String> {
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
            let index = IndexWithTask {
                title: "SingleTask".into(),
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
            let index = IndexWithTask {
                title: "SingleTask".into(),
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

fn get_content_color_class(task: &Task) -> String {
    match task.priority {
        Priority::None => String::new(),
        Priority::Low => String::from("has-text-primary"),
        Priority::Medium => String::from("has-text-warning"),
        Priority::High => String::from("has-text-danger"),
    }
}
