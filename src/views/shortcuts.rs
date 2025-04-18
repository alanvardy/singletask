use crate::error::Error;
use crate::unsplash;
use crate::unsplash::Unsplash;
use crate::{AppState, Link};
use askama::Template;
use axum::extract::State;
use axum::{extract::Query, response::Html, routing::get, Router};
use std::collections::HashMap;
use std::sync::Arc;

pub fn routes(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/shortcuts", get(shortcuts))
        .with_state(app_state)
}

#[derive(Template)]
#[template(path = "shortcuts.html")]
struct ShortcutsTemplate {
    title: String,
    navigation: Vec<Link>,
    unsplash: Unsplash,
}

async fn shortcuts(
    State(_app_state): State<Arc<AppState>>,
    Query(_params): Query<HashMap<String, String>>,
) -> Result<Html<String>, Error> {
    let index = ShortcutsTemplate {
        title: "Keyboard Shortcuts".into(),
        navigation: crate::get_nav(),
        unsplash: unsplash::stub(),
    };

    Ok(Html(index.render()?))
}
