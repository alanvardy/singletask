use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use std::num::ParseIntError;
use tokio::task::JoinError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub message: String,
    pub source: String,
    pub code: StatusCode,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let Self {
            code,
            source,
            message,
        } = self;
        (code, format!("{source}: {message}")).into_response()
    }
}

impl From<askama::Error> for Error {
    fn from(value: askama::Error) -> Self {
        match value {
            askama::Error::Fmt => Self {
                source: String::from("askama_axum"),
                message: "Unknown formatting error".to_string(),
                code: StatusCode::INTERNAL_SERVER_ERROR,
            },
            askama::Error::Custom(error) => Self {
                source: String::from("askama_axum"),
                message: format!("Error on ? in template {error}"),
                code: StatusCode::INTERNAL_SERVER_ERROR,
            },
            _ => Self {
                source: String::from("askama_axum"),
                message: value.to_string(),
                code: StatusCode::INTERNAL_SERVER_ERROR,
            },
        }
    }
}

impl From<echodb::Error> for Error {
    fn from(value: echodb::Error) -> Self {
        Self {
            source: String::from("echodb"),
            message: format!("{value}"),
            code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<chrono::LocalResult<chrono::DateTime<chrono_tz::Tz>>> for Error {
    fn from(value: chrono::LocalResult<chrono::DateTime<chrono_tz::Tz>>) -> Self {
        Self {
            source: String::from("chrono"),
            message: format!("{value:?}"),
            code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<tokio::sync::mpsc::error::SendError<Error>> for Error {
    fn from(value: tokio::sync::mpsc::error::SendError<Error>) -> Self {
        Self {
            source: String::from("tokio mpsc"),
            message: format!("{value}"),
            code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<chrono_tz::ParseError> for Error {
    fn from(value: chrono_tz::ParseError) -> Self {
        Self {
            source: String::from("chrono_tz"),
            message: format!("{value}"),
            code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<JoinError> for Error {
    fn from(value: JoinError) -> Self {
        Self {
            source: String::from("tokio JoinError"),
            message: format!("{value}"),
            code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<ParseIntError> for Error {
    fn from(value: ParseIntError) -> Self {
        Self {
            source: String::from("ParseIntError"),
            message: format!("{value}"),
            code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<chrono::ParseError> for Error {
    fn from(value: chrono::ParseError) -> Self {
        Self {
            source: String::from("chrono"),
            message: format!("{value}"),
            code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self {
            source: String::from("serde_json"),
            message: format!("{value}"),
            code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        Self {
            source: String::from("reqwest"),
            message: format!("{value}"),
            code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub fn new(source: &str, message: &str) -> Error {
    Error {
        source: String::from(source),
        message: String::from(message),
        code: StatusCode::INTERNAL_SERVER_ERROR,
    }
}
