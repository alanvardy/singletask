use crate::request;
use crate::time;
use crate::Env;
use crate::UserState;
use crate::{error::Error, AppState};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const MAX_UNSPLASH_AGE_MIN: i64 = 1440;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Unsplash {
    pub urls: Urls,
    pub links: Links,
    pub user: User,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Urls {
    pub full: String,
    pub regular: String,
    pub small: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Links {
    pub html: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct User {
    pub name: String,
}

/// Fetches from cache or API
pub async fn cached_get_random(
    app_state: &Arc<AppState>,
    user_state: &UserState,
    timezone: &Tz,
    key: String,
) -> Result<Unsplash, Error> {
    let updated_at = user_state.unsplash_updated_at;
    let unsplash = user_state.unsplash.clone();

    if unsplash.is_some()
        && updated_at.is_some()
        && time::age_in_minutes(updated_at.unwrap(), timezone)? < MAX_UNSPLASH_AGE_MIN
    {
        Ok(unsplash.clone().unwrap())
    } else {
        let unsplash = get_random(app_state).await?;

        let db = &app_state.clone().db;
        let mut tx = db.begin(true).await?;
        let user_state = UserState {
            unsplash: Some(unsplash.clone()),
            unsplash_updated_at: Some(time::now(timezone)?),
            ..user_state.clone()
        };
        tx.set(key.clone(), user_state)?;
        tx.commit()?;

        Ok(unsplash)
    }
}

pub async fn get_random(app_state: &Arc<AppState>) -> Result<Unsplash, Error> {
    match app_state.env {
        Env::Prod => {
            let api_key = app_state.unsplash_api_key.clone();
            let json = request::get_random_unsplash(api_key).await?;
            json_to_unsplash(json)
        }
        Env::Dev => Ok(stub()),
        Env::Test => Ok(stub()),
    }
}

pub fn json_to_unsplash(json: String) -> Result<Unsplash, Error> {
    let unsplash: Unsplash = serde_json::from_str(&json)?;
    Ok(unsplash)
}

pub fn stub() -> Unsplash {
    Unsplash {
            urls: Urls {
                full: "https://images.unsplash.com/photo-1731453171960-0f8c884c72a4?crop=entropy&cs=srgb&fm=jpg&ixid=M3w3NDgzNHwwfDF8cmFuZG9tfHx8fHx8fHx8MTczMzAzMDYxOHw&ixlib=rb-4.0.3&q=85".to_string(),
                regular: "https://images.unsplash.com/photo-1731453171960-0f8c884c72a4?crop=entropy&cs=tinysrgb&fit=max&fm=jpg&ixid=M3w3NDgzNHwwfDF8cmFuZG9tfHx8fHx8fHx8MTczMzAzMDYxOHw&ixlib=rb-4.0.3&q=80&w=1080".to_string(),
                small: "https://images.unsplash.com/photo-1731453171960-0f8c884c72a4?crop=entropy&cs=tinysrgb&fit=max&fm=jpg&ixid=M3w3NDgzNHwwfDF8cmFuZG9tfHx8fHx8fHx8MTczMzAzMDYxOHw&ixlib=rb-4.0.3&q=80&w=400".to_string(),
            },
            links: Links {
                html: "https://unsplash.com/photos/a-blurry-photo-of-a-beach-at-sunset-Qn2nubHzL7w"
                    .to_string(),
            },
            user: User {
                name: "Adrian Botica".to_string(),
            },
        }
}
