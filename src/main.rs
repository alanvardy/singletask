use axum::Router;
use chrono::DateTime;
use chrono_tz::Tz;
use echodb::Db;
use serde::Serialize;
use tasks::Task;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

mod error;
mod index;
mod tasks;
mod time;

struct AppState {
    db: Db<String, UserState>,
}

#[derive(Clone, Eq, PartialEq)]
struct UserState {
    tasks: Vec<Task>,
    updated_at: DateTime<Tz>,
}

#[derive(Serialize)]
struct Link {
    name: String,
    href: String,
}

fn routes() -> Router {
    Router::new()
        // Routes
        .merge(index::routes())
}

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    let router = routes().layer(
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
    use super::*;
    use axum_test::TestServer;

    #[tokio::test]
    async fn test_index() {
        // you can replace this Router with your own app

        let server = TestServer::new(routes()).unwrap();
        // Get the request.
        let response = server.get("/").await;

        assert!(response.text().contains("Todoist"))
    }
}
