use chrono_tz::Tz;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use crate::{error::Error, request, time, AppState, UserState};

const SYNC_URL: &str = "/sync/v9/sync";

/// https://developer.todoist.com/sync/v9/#user
#[derive(Deserialize, Debug)]
pub struct SyncResponse {
    user: User,
}

#[derive(Deserialize, Debug)]
pub struct User {
    tz_info: TzInfo,
}

#[derive(Deserialize, Debug)]
pub struct TzInfo {
    timezone: String,
}

/// Fetches from cache or API
pub async fn cached_get_timezone(
    app_state: &Arc<AppState>,
    user_state: &UserState,
    token: &str,
    key: &str,
) -> Result<Tz, Error> {
    if let Some(timezone) = user_state.timezone {
        Ok(timezone)
    } else {
        let User {
            tz_info: TzInfo { timezone },
        } = get_user_data(token).await?;
        let tz = time::timezone_from_str(&timezone)?;

        let db = &app_state.clone().db;
        let mut tx = db.begin(true).await;
        let user_state = UserState {
            timezone: Some(tz),
            ..user_state.clone()
        };
        tx.set(key.to_string(), user_state)?;
        tx.commit()?;

        Ok(tz)
    }
}

pub async fn get_user_data(token: &str) -> Result<User, Error> {
    let url = SYNC_URL.to_string();
    let body = json!({"resource_types": ["user"], "sync_token": "*"});
    let json = request::post_todoist_sync(token, &url, body).await?;
    sync_json_to_user(json)
}

pub fn sync_json_to_user(json: String) -> Result<User, Error> {
    let sync_response: SyncResponse = serde_json::from_str(&json)?;
    Ok(sync_response.user)
}
