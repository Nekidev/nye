use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::registries::http::server::v1::schemas::ErrorSchema;

#[derive(Debug, Clone)]
pub struct Error {
    pub status: StatusCode,
    pub title: String,
    pub message: String,
}

impl Error {
    pub fn new(
        status: impl Into<StatusCode>,
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            status: status.into(),
            title: title.into(),
            message: message.into(),
        }
    }

    pub fn new_401(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            title: title.into(),
            message: message.into(),
        }
    }

    pub fn new_409(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            title: title.into(),
            message: message.into(),
        }
    }

    pub fn new_418(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::IM_A_TEAPOT,
            title: title.into(),
            message: message.into(),
        }
    }

    pub fn new_422(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            title: title.into(),
            message: message.into(),
        }
    }

    pub fn http_429() -> Self {
        Self {
            status: StatusCode::TOO_MANY_REQUESTS,
            title: "Too Many Requests".into(),
            message: "You have made too many requests, chill.".into(),
        }
    }

    pub fn http_500() -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: "Internal Server Error".into(),
            message: "An internal server error occurred while attempting to process your request."
                .into(),
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        (
            self.status,
            Json(ErrorSchema {
                title: self.title,
                message: self.message,
            }),
        )
            .into_response()
    }
}

pub trait OrHttpError<T> {
    fn or_http_500(self) -> Result<T, Error>;
}

impl<T, E> OrHttpError<T> for Result<T, E> {
    fn or_http_500(self) -> Result<T, Error> {
        match self {
            Ok(v) => Ok(v),
            Err(_) => Err(Error::http_500()),
        }
    }
}
