use std::sync::Arc;

use axum::Router;
use chrono::DateTime;
use chrono_tz::Tz;
use echodb::Database;
use serde::Serialize;
use shuttle_runtime::SecretStore;
use std::str::FromStr;
use strum::EnumString;
use tasks::Task;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;
use unsplash::Unsplash;

mod error;
mod request;
mod responses;
mod tasks;
mod time;
mod unsplash;
mod user;
mod views;

const UNSPLASH_API_KEY: &str = "UNSPLASH_API_KEY";
const ENV: &str = "ENV";

struct AppState {
    db: Database<String, UserState>,
    unsplash_api_key: String,
    env: Env,
    test_server_url: Option<String>,
}

#[derive(EnumString)]
enum Env {
    Prod,
    Dev,
    Test,
}

#[derive(Clone, Eq, PartialEq, Debug)]
struct UserState {
    tasks: Vec<Task>,
    skip_task_ids: Vec<String>,
    tasks_updated_at: Option<DateTime<Tz>>,
    unsplash: Option<Unsplash>,
    unsplash_updated_at: Option<DateTime<Tz>>,
    timezone: Option<Tz>,
}

#[derive(Serialize)]
struct Link {
    name: String,
    href: String,
}

fn routes(app_state: Arc<AppState>) -> Router {
    Router::new()
        // Routes
        .merge(views::index::routes(app_state.clone()))
        .merge(views::shortcuts::routes(app_state.clone()))
        .merge(views::process::routes(app_state))
}

#[shuttle_runtime::main]
async fn main(#[shuttle_runtime::Secrets] secrets: SecretStore) -> shuttle_axum::ShuttleAxum {
    let db = echodb::new::<String, UserState>();
    let unsplash_api_key = secrets.get(UNSPLASH_API_KEY).expect(UNSPLASH_API_KEY);
    let env = secrets.get(ENV).expect(ENV);
    let app_state = Arc::new(AppState {
        db,
        unsplash_api_key,
        env: Env::from_str(&env).unwrap(),
        test_server_url: None,
    });

    let router = routes(app_state).layer(
        TraceLayer::new_for_http()
            .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
            .on_response(DefaultOnResponse::new().level(Level::INFO)),
    );

    Ok(router.into())
}

fn get_nav() -> Vec<Link> {
    vec![
        Link {
            href: "/".into(),
            name: "SingleTask".into(),
        },
        Link {
            href: "/shortcuts".into(),
            name: "Keyboard Shortcuts".into(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use crate::responses::ResponseFromFile;

    use super::*;
    use axum_test::TestServer;

    #[tokio::test]
    async fn test_home() {
        let db = echodb::new::<String, UserState>();
        let app_state = Arc::new(AppState {
            db,
            unsplash_api_key: "123".to_string(),
            env: Env::Test,
            test_server_url: None,
        });
        let server = TestServer::new(routes(app_state)).unwrap();

        let url = "/";
        let text = "Todoist";

        let response = server.get(url).await;
        assert!(response.text().contains(text))
    }

    #[tokio::test]
    async fn test_process() {
        let db = echodb::new::<String, UserState>();
        let mut server = mockito::Server::new_async().await;
        let url = "/process?token=xxxx&filter=%23checklist&timezone=America%2FLos_Angeles";
        let mock = server
            .mock("POST", "/sync/v9/sync")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(ResponseFromFile::Sync.read().await)
            .create_async()
            .await;
        let mock2 = server
            .mock("GET", "/rest/v2/tasks/?filter=%23checklist")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(ResponseFromFile::Tasks.read().await)
            .create_async()
            .await;
        let app_state = Arc::new(AppState {
            db,
            unsplash_api_key: "123".to_string(),
            env: Env::Test,
            test_server_url: Some(server.url()),
        });
        let server = TestServer::new(routes(app_state)).unwrap();

        let text = "Change water filter under sink";

        let response = server.get(url).await;
        assert!(response.text().contains(text));
        mock.assert();
        mock2.assert();
    }
}
