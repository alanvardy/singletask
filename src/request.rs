use crate::error;
use crate::error::Error;
use reqwest::header::AUTHORIZATION;
use reqwest::header::CONTENT_TYPE;
use reqwest::Client;
use reqwest::Response;
use serde_json::json;

const TODOIST_URL: &str = "https://api.todoist.com";
const UNSPLASH_URL: &str = "https://api.unsplash.com/photos/random?query=nature";
const ACCEPT_VERSION: &str = "Accept-Version";
const UNSPLASH_VERSION: &str = "v1";

/// Get Todoist via REST api
pub async fn get_todoist_rest(token: &str, url: &str) -> Result<String, Error> {
    let request_url = format!("{TODOIST_URL}{url}");
    let authorization: &str = &format!("Bearer {token}");
    let response = Client::new()
        .get(request_url.clone())
        .header(CONTENT_TYPE, "application/json")
        .header(AUTHORIZATION, authorization)
        .send()
        .await?;

    handle_response(response, "GET", url, json!({})).await
}

/// Post to Todoist via sync API
/// We use sync when we want natural languague processing.
pub async fn post_todoist_sync(
    token: &str,
    url: &str,
    body: serde_json::Value,
) -> Result<String, Error> {
    let request_url = format!("{TODOIST_URL}{url}");

    let response = Client::new()
        .post(request_url.clone())
        .header(CONTENT_TYPE, "application/json")
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .json(&body)
        .send()
        .await?;

    handle_response(response, "POST", url, body).await
}

pub async fn get_random_unsplash(api_key: String) -> Result<String, Error> {
    let url = UNSPLASH_URL.to_string();
    let authorization = format!("Client-ID {api_key}");
    let response = Client::new()
        .get(url.clone())
        .header(ACCEPT_VERSION, UNSPLASH_VERSION)
        .header(CONTENT_TYPE, "application/json")
        .header(AUTHORIZATION, authorization)
        .send()
        .await?;

    handle_response(response, "GET", &url, json!({})).await
}

async fn handle_response(
    response: Response,
    method: &str,
    url: &str,
    body: serde_json::Value,
) -> Result<String, Error> {
    if response.status().is_success() {
        Ok(response.text().await?)
    } else {
        let json_string = response.text().await?;
        Err(error::new(
            "reqwest",
            &format!(
                "
            method: {method}
            url: {url}
            body: {body}
            response: {json_string}",
            ),
        ))
    }
}
