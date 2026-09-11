use std::fmt::Display;

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

    pub fn http_400() -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            title: "Bad Request".into(),
            message: "The request you sent was invalid.".into(),
        }
    }

    pub fn http_401() -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            title: "Unauthorized".into(),
            message: "This action requires authorization. Did you send an Authorization header? Was it valid? Did the token expire? Right token type?".into(),
        }
    }

    pub fn http_422() -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            title: "Invalid Request".into(),
            message:
                "The request you made was not valid. Review the documentation before retrying."
                    .into(),
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

pub trait OptionOrHttpError<T> {
    /// Replaces [`None`] with a generic 400 [`Error`].
    fn or_http_400(self) -> Result<T, Error>;
    /// Replaces [`None`] with a generic 401 [`Error`].
    fn or_http_401(self) -> Result<T, Error>;
    /// Replaces [`None`] with a generic 422 [`Error`].
    fn or_http_422(self) -> Result<T, Error>;
    /// Replaces [`None`] with a generic 500 [`Error`].
    fn or_http_500(self) -> Result<T, Error>;
}

impl<T> OptionOrHttpError<T> for Option<T> {
    fn or_http_400(self) -> Result<T, Error> {
        match self {
            Some(t) => Ok(t),
            None => Err(Error::http_400()),
        }
    }

    fn or_http_401(self) -> Result<T, Error> {
        match self {
            Some(t) => Ok(t),
            None => Err(Error::http_401()),
        }
    }

    fn or_http_422(self) -> Result<T, Error> {
        match self {
            Some(t) => Ok(t),
            None => Err(Error::http_422()),
        }
    }

    fn or_http_500(self) -> Result<T, Error> {
        match self {
            Some(t) => Ok(t),
            None => Err(Error::http_500()),
        }
    }
}

pub trait ResultOrHttpError<T> {
    /// Converts the error of the result to an [`Error`] using the error as the
    /// returned error's message.
    fn or_into_422(self) -> Result<T, Error>;
    /// Replaces the error of the result with a generic 400 [`Error`].
    fn or_http_400(self) -> Result<T, Error>;
    /// Replaces the error of the result with a generic 422 [`Error`].
    fn or_http_422(self) -> Result<T, Error>;
    /// Replaces the error of the result with a generic 500 [`Error`].
    fn or_http_500(self) -> Result<T, Error>;
}

impl<T, E> ResultOrHttpError<T> for Result<T, E>
where
    E: Display,
{
    fn or_into_422(self) -> Result<T, Error> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(Error::new_422("Invalid Request", e.to_string())),
        }
    }

    fn or_http_400(self) -> Result<T, Error> {
        match self {
            Ok(v) => Ok(v),
            Err(_) => Err(Error::http_400()),
        }
    }

    fn or_http_422(self) -> Result<T, Error> {
        match self {
            Ok(v) => Ok(v),
            Err(_) => Err(Error::http_422()),
        }
    }

    fn or_http_500(self) -> Result<T, Error> {
        match self {
            Ok(v) => Ok(v),
            Err(_) => Err(Error::http_500()),
        }
    }
}
