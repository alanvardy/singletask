use crate::Link;
use askama_axum::Template;
use axum::{extract::Path, response::Html, routing::get, Router};

pub fn routes() -> Router {
    Router::new()
        .route("/", get(index))
        .route("/:token", get(index_with_token))
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    title: String,
    navigation: Vec<Link>,
}

async fn index() -> Html<String> {
    let index = IndexTemplate {
        title: "SingleTask".into(),
        navigation: crate::get_nav(),
    };

    Html(index.render().unwrap())
}

#[derive(Template)]
#[template(path = "index_with_token.html")]
struct IndexWithTokenTemplate {
    title: String,
    navigation: Vec<Link>,
    token: String,
}

async fn index_with_token(Path(token): Path<String>) -> Html<String> {
    let index = IndexWithTokenTemplate {
        title: "SingleTask".into(),
        navigation: crate::get_nav(),
        token,
    };

    Html(index.render().unwrap())
}
