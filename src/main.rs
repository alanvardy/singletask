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
    use super::*;
    use axum_test::TestServer;

    #[tokio::test]
    async fn test_home() {
        let db = echodb::new::<String, UserState>();
        let app_state = Arc::new(AppState {
            db,
            unsplash_api_key: "123".to_string(),
            env: Env::Test,
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
        let app_state = Arc::new(AppState {
            db,
            unsplash_api_key: "123".to_string(),
            env: Env::Test,
        });
        let server = TestServer::new(routes(app_state)).unwrap();

        let url = "/process?token=a5c4e1bc54e1c79aca0c7b8bf57c4ed2b99ba608&filter=%23checklist&timezone=America%2FLos_Angeles";
        let text = "Would you like to use another filter?";

        let response = server.get(url).await;
        assert!(response.text().contains(text))
    }
}
