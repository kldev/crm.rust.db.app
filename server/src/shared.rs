//! Small shared pieces of the API: keyset cursors, pages, JSON bodies with validation, timestamps.

use axum::{
    extract::{FromRequest, FromRequestParts, Path, Query, Request, rejection::JsonRejection},
    http::StatusCode,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use serde::{Serialize, Serializer, de::DeserializeOwned};
use utoipa::ToSchema;

use crate::error::{ApiError, ApiResult};

/// One page of a keyset-paged list. `nextCursor` is null on the last page; pass it back as `cursor`.
/// One concrete type per item type, named as the web app's generated client knows it (`PageResponseUserResponse`).
#[macro_export]
macro_rules! page_response {
    ($name:ident, $item:ty) => {
        #[derive(serde::Serialize, utoipa::ToSchema)]
        #[serde(rename_all = "camelCase")]
        pub struct $name {
            pub items: Vec<$item>,
            pub next_cursor: Option<String>,
        }

        impl $name {
            pub fn new((items, next_cursor): (Vec<$item>, Option<String>)) -> Self {
                $name { items, next_cursor }
            }
        }
    };
}

/// Number of rows matching a list's filters (asked for separately: counting reads every matching row).
#[derive(Serialize, ToSchema)]
pub struct TotalResponse {
    pub total: i64,
}

const SEPARATOR: char = '\u{1F}';

/// Opaque keyset cursors: the values of the last row of a page, base64url-encoded (Paging.kt).
pub mod cursor {
    use super::*;

    pub fn encode(parts: &[&dyn ToString]) -> String {
        let joined = parts
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(&SEPARATOR.to_string());
        URL_SAFE_NO_PAD.encode(joined)
    }

    pub fn decode(cursor: Option<&str>, expected_parts: usize) -> ApiResult<Option<Vec<String>>> {
        let Some(cursor) = cursor.filter(|c| !c.trim().is_empty()) else {
            return Ok(None);
        };
        let invalid = || ApiError::bad_request("Invalid cursor.");
        let bytes = URL_SAFE_NO_PAD
            .decode(cursor.trim_end_matches('='))
            .map_err(|_| invalid())?;
        let text = String::from_utf8(bytes).map_err(|_| invalid())?;
        let parts: Vec<String> = text.split(SEPARATOR).map(str::to_owned).collect();
        if parts.len() != expected_parts {
            return Err(invalid());
        }
        Ok(Some(parts))
    }

    /// Cuts the extra row the functions return (page_size + 1) and builds the next cursor from the last item.
    pub fn page<T>(
        mut rows: Vec<T>,
        page_size: i32,
        cursor_of: impl Fn(&T) -> String,
    ) -> (Vec<T>, Option<String>) {
        let has_more = rows.len() > page_size as usize;
        if has_more {
            rows.truncate(page_size as usize);
        }
        let next_cursor = if has_more {
            rows.last().map(cursor_of)
        } else {
            None
        };
        (rows, next_cursor)
    }
}

/// `size` of a list: 1-200, 50 by default (the @Min/@Max of the controllers).
pub fn page_size(size: Option<i32>) -> ApiResult<i32> {
    let size = size.unwrap_or(50);
    if !(1..=200).contains(&size) {
        return Err(ApiError::bad_request("size: must be between 1 and 200"));
    }
    Ok(size)
}

/// Request bodies check their own fields (the Bean Validation annotations of the Kotlin DTOs).
pub trait Validate {
    fn validate(&self, v: &mut Checks);
}

/// Collects field errors; one 400 lists all of them.
#[derive(Default)]
pub struct Checks(Vec<String>);

impl Checks {
    pub fn fail(&mut self, field: &str, message: &str) {
        self.0.push(format!("{field}: {message}"));
    }

    /// @NotBlank + @Size(min, max).
    pub fn text(&mut self, field: &str, value: &Option<String>, min: usize, max: usize) {
        match value.as_deref() {
            None => self.fail(field, "must not be blank"),
            Some(v) if v.trim().is_empty() => self.fail(field, "must not be blank"),
            Some(v) => {
                let n = v.chars().count();
                if n < min || n > max {
                    self.fail(field, &format!("size must be between {min} and {max}"));
                }
            }
        }
    }

    /// @NotBlank + @Pattern(one of).
    pub fn one_of(&mut self, field: &str, value: &Option<String>, allowed: &[&str]) {
        match value.as_deref() {
            Some(v) if allowed.contains(&v) => {}
            Some(v) if !v.trim().is_empty() => {
                self.fail(field, &format!("must match \"{}\"", allowed.join("|")))
            }
            _ => self.fail(field, "must not be blank"),
        }
    }

    pub fn present<T>(&mut self, field: &str, value: &Option<T>) {
        if value.is_none() {
            self.fail(field, "must not be null");
        }
    }

    /// @Size(max) of an optional text.
    pub fn max(&mut self, field: &str, value: &Option<String>, max: usize) {
        if value.as_deref().is_some_and(|v| v.chars().count() > max) {
            self.fail(field, &format!("size must be between 0 and {max}"));
        }
    }

    /// The `rowVersion` read from the server: 16 hex digits. `required` = @NotBlank.
    pub fn row_version(&mut self, value: &Option<String>, required: bool) {
        match value.as_deref() {
            None | Some("") if required => self.fail("rowVersion", "must not be blank"),
            None => {}
            Some(v) if v.len() == 16 && v.chars().all(|c| c.is_ascii_hexdigit()) => {}
            Some(_) => self.fail("rowVersion", "must match \"^[0-9a-fA-F]{16}$\""),
        }
    }

    /// @NotNull @DecimalMin("0") @Digits(integer = 17, fraction = 2).
    pub fn money(&mut self, field: &str, value: &Option<rust_decimal::Decimal>) {
        match value {
            None => self.fail(field, "must not be null"),
            Some(v) if v.is_sign_negative() && !v.is_zero() => {
                self.fail(field, "must be greater than or equal to 0")
            }
            Some(v)
                if v.normalize().scale() > 2
                    || v.trunc().abs() >= rust_decimal::Decimal::from(10i64.pow(17)) =>
            {
                self.fail(
                    field,
                    "numeric value out of bounds (<17 digits>.<2 digits> expected)",
                )
            }
            Some(_) => {}
        }
    }

    pub fn into_result(self) -> ApiResult<()> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(ApiError::bad_request(self.0.join("; ")))
        }
    }
}

/// JSON body: a malformed body is 400 (not axum's 422), then the body's own field checks.
pub struct Body<T>(pub T);

impl<S, T> FromRequest<S> for Body<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let axum::Json(value) =
            axum::Json::<T>::from_request(req, state)
                .await
                .map_err(|e: JsonRejection| match e.status() {
                    StatusCode::UNSUPPORTED_MEDIA_TYPE => ApiError::Problem {
                        status: StatusCode::UNSUPPORTED_MEDIA_TYPE,
                        title: "Unsupported Media Type",
                        detail: "Content-Type must be application/json.".into(),
                        error_number: None,
                    },
                    _ => ApiError::bad_request("Failed to read request."),
                })?;
        let mut checks = Checks::default();
        value.validate(&mut checks);
        checks.into_result()?;
        Ok(Body(value))
    }
}

/// Query string: a value of the wrong type is 400 ProblemDetail (axum's own rejection is plain text).
pub struct Params<T>(pub T);

impl<S, T> FromRequestParts<S> for Params<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        Query::<T>::from_request_parts(parts, state)
            .await
            .map(|Query(v)| Params(v))
            .map_err(|e| ApiError::bad_request(e.body_text()))
    }
}

/// Path parameters: a value of the wrong type (`/api/clients/abc`) is 400 ProblemDetail.
pub struct PathParam<T>(pub T);

impl<S, T> FromRequestParts<S> for PathParam<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Send,
{
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        Path::<T>::from_request_parts(parts, state)
            .await
            .map(|Path(v)| PathParam(v))
            .map_err(|e| ApiError::bad_request(e.body_text()))
    }
}

/// Timestamps: the API kept two forms from the Kotlin version and the web app relies on both.
pub mod ts {
    use super::*;

    /// A LocalDateTime in UTC without the zone, e.g. `2026-11-08T10:15:30.123` (the app appends `Z`).
    pub fn local<S: Serializer>(value: &DateTime<Utc>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&value.naive_utc().format("%Y-%m-%dT%H:%M:%S%.f").to_string())
    }

    pub fn local_opt<S: Serializer>(
        value: &Option<DateTime<Utc>>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(v) => local(v, s),
            None => s.serialize_none(),
        }
    }

    /// An Instant, e.g. `2026-11-08T10:15:30.123Z` (the service desk).
    pub fn instant<S: Serializer>(value: &DateTime<Utc>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&value.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true))
    }

    pub fn instant_opt<S: Serializer>(
        value: &Option<DateTime<Utc>>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(v) => instant(v, s),
            None => s.serialize_none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Item(i32);

    #[test]
    fn cursor_round_trip_and_rejects_garbage() {
        let c = cursor::encode(&[&42, &"Zielony"]);
        assert_eq!(
            cursor::decode(Some(&c), 2).unwrap().unwrap(),
            vec!["42", "Zielony"]
        );
        assert!(cursor::decode(Some(&c), 1).is_err());
        assert!(cursor::decode(Some("!!!"), 1).is_err());
        assert!(cursor::decode(None, 1).unwrap().is_none());
    }

    #[test]
    fn page_cuts_the_extra_row() {
        let (items, next) = cursor::page(vec![Item(1), Item(2), Item(3)], 2, |i| i.0.to_string());
        assert_eq!(items.len(), 2);
        assert_eq!(next.as_deref(), Some("2"));
        let (_, last) = cursor::page(vec![Item(1)], 2, |i| i.0.to_string());
        assert!(last.is_none());
    }
}
