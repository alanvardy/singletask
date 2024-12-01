use axum::Router;
use chrono::DateTime;
use chrono_tz::Tz;
use echodb::Db;
use serde::Serialize;
use shuttle_runtime::SecretStore;
use strum::EnumString;
use tasks::Task;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;
use unsplash::Unsplash;

mod error;
mod index;
mod request;
mod tasks;
mod time;
mod unsplash;

struct AppState {
    db: Db<String, UserState>,
    unsplash_api_key: String,
    env: Env,
}

#[derive(EnumString)]
enum Env {
    Prod,
    Dev,
}

#[derive(Clone, Eq, PartialEq)]
struct UserState {
    tasks: Vec<Task>,
    skip_task_ids: Vec<String>,
    tasks_updated_at: DateTime<Tz>,
    unsplash: Option<Unsplash>,
    unsplash_updated_at: DateTime<Tz>,
}

#[derive(Serialize)]
struct Link {
    name: String,
    href: String,
}

fn routes(secrets: SecretStore) -> Router {
    Router::new()
        // Routes
        .merge(index::routes(secrets))
}

#[shuttle_runtime::main]
async fn main(#[shuttle_runtime::Secrets] secrets: SecretStore) -> shuttle_axum::ShuttleAxum {
    let router = routes(secrets).layer(
        TraceLayer::new_for_http()
            .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
            .on_response(DefaultOnResponse::new().level(Level::INFO)),
    );
    Ok(router.into())
}

fn get_nav() -> Vec<Link> {
    vec![Link {
        href: "/".into(),
        name: "SingleTask".into(),
    }]
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use axum_test::TestServer;

    #[tokio::test]
    async fn test_home() {
        // you can replace this Router with your own app

        let bt = BTreeMap::from([
            ("UNSPLASH_API_KEY".to_owned(), "2".to_owned().into()),
            ("ENV".to_owned(), "Dev".to_owned().into()),
        ]);
        let ss = SecretStore::new(bt);
        let server = TestServer::new(routes(ss)).unwrap();
        // Get the request.
        let response = server.get("/").await;

        assert!(response.text().contains("Todoist"))
    }
}
