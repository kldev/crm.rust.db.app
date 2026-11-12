//! Errors as RFC 9457 ProblemDetail (`application/problem+json`), the body the web app reads
//! (`frontend/src/api/mutator.ts`: `title`, `detail`, `status`, `errorNumber`).
//!
//! Database rules arrive as SQLSTATE `U<4 digits>` raised by the procedures (ADR-0013) and leave the API as the
//! same `errorNumber` the T-SQL version used: `U1010` -> 51010, `U0001` -> 50001.

use axum::{
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Debug)]
pub enum ApiError {
    /// A ProblemDetail body.
    Problem {
        status: StatusCode,
        title: &'static str,
        detail: String,
        error_number: Option<i32>,
    },
    /// Status only, empty body (what Spring Security answered for 401 and 403).
    Status(StatusCode),
    /// A bug or an unavailable dependency: logged, answered without details.
    Internal(anyhow::Error),
}

pub type ApiResult<T> = Result<T, ApiError>;

impl ApiError {
    pub fn bad_request(detail: impl Into<String>) -> Self {
        ApiError::Problem {
            status: StatusCode::BAD_REQUEST,
            title: "Validation failed",
            detail: detail.into(),
            error_number: None,
        }
    }

    pub fn not_found(detail: impl Into<String>) -> Self {
        ApiError::Problem {
            status: StatusCode::NOT_FOUND,
            title: "Not found",
            detail: detail.into(),
            error_number: None,
        }
    }

    pub fn unauthorized() -> Self {
        ApiError::Status(StatusCode::UNAUTHORIZED)
    }

    pub fn forbidden() -> Self {
        ApiError::Status(StatusCode::FORBIDDEN)
    }
}

/// HTTP status, title and whether the database message may be shown, for an error number (ProcedureErrors.kt).
fn mapping(number: i32) -> (StatusCode, &'static str, bool) {
    match number {
        51000..=51299 => (StatusCode::BAD_REQUEST, "Validation failed", true),
        51300..=51399 => (StatusCode::FORBIDDEN, "Forbidden", true),
        51400..=51499 => (StatusCode::NOT_FOUND, "Not found", true),
        51900..=51999 => (StatusCode::CONFLICT, "Conflict", true),
        2601 | 2627 => (StatusCode::CONFLICT, "Already exists", false),
        547 => (StatusCode::BAD_REQUEST, "Invalid reference", false),
        1205 => (StatusCode::SERVICE_UNAVAILABLE, "Please retry", false),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "Database error", false),
    }
}

/// The error number for a SQLSTATE: our own `Uxxxx`, or the PostgreSQL states that had a T-SQL number.
pub fn error_number(sqlstate: &str) -> Option<i32> {
    if let Some(digits) = sqlstate.strip_prefix('U')
        && digits.len() == 4
        && let Ok(n) = digits.parse::<i32>()
    {
        return Some(50_000 + n);
    }
    match sqlstate {
        "23505" => Some(2627),           // unique_violation
        "23503" => Some(547),            // foreign_key_violation
        "40P01" | "40001" => Some(1205), // deadlock, serialization failure
        _ => None,
    }
}

impl From<tokio_postgres::Error> for ApiError {
    fn from(error: tokio_postgres::Error) -> Self {
        let Some(db) = error.as_db_error() else {
            return ApiError::Internal(error.into());
        };
        let Some(number) = error_number(db.code().code()) else {
            return ApiError::Internal(anyhow::anyhow!(
                "database error {}: {}",
                db.code().code(),
                db.message()
            ));
        };
        let (status, title, expose) = mapping(number);
        if status.is_server_error() {
            tracing::error!(
                sqlstate = db.code().code(),
                message = db.message(),
                "database call failed"
            );
        }
        ApiError::Problem {
            status,
            title,
            detail: if expose {
                db.message().to_owned()
            } else {
                title.to_owned()
            },
            error_number: Some(number).filter(|n| expose || matches!(n, 2601 | 2627 | 547 | 1205)),
        }
    }
}

impl From<deadpool_postgres::PoolError> for ApiError {
    fn from(error: deadpool_postgres::PoolError) -> Self {
        ApiError::Internal(anyhow::anyhow!("database pool: {error}"))
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(error: anyhow::Error) -> Self {
        ApiError::Internal(error)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProblemBody<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    title: &'a str,
    status: u16,
    detail: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_number: Option<i32>,
}

pub fn problem(
    status: StatusCode,
    title: &str,
    detail: &str,
    error_number: Option<i32>,
) -> Response {
    let body = ProblemBody {
        kind: "about:blank",
        title,
        status: status.as_u16(),
        detail,
        error_number,
    };
    (
        status,
        [(header::CONTENT_TYPE, "application/problem+json")],
        serde_json::to_vec(&body).unwrap_or_default(),
    )
        .into_response()
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::Problem {
                status,
                title,
                detail,
                error_number,
            } => problem(status, title, &detail, error_number),
            ApiError::Status(status) => status.into_response(),
            ApiError::Internal(error) => {
                tracing::error!(error = format!("{error:#}"), "request failed");
                problem(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal error",
                    "Something went wrong.",
                    None,
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlstates_become_the_old_error_numbers() {
        assert_eq!(error_number("U1010"), Some(51010));
        assert_eq!(error_number("U1913"), Some(51913));
        assert_eq!(error_number("U0001"), Some(50001));
        assert_eq!(error_number("23505"), Some(2627));
        assert_eq!(error_number("42501"), None);
        assert_eq!(mapping(51018).0, StatusCode::BAD_REQUEST);
        assert_eq!(mapping(51301).0, StatusCode::FORBIDDEN);
        assert_eq!(mapping(51402).0, StatusCode::NOT_FOUND);
        assert_eq!(mapping(51901).0, StatusCode::CONFLICT);
        assert_eq!(mapping(50001).0, StatusCode::INTERNAL_SERVER_ERROR);
    }
}
