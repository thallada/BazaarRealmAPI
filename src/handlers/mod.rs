use anyhow::{anyhow, Result};
use http::header::{HeaderValue, CONTENT_TYPE, ETAG};
use http::StatusCode;
use http_api_problem::HttpApiProblem;
use seahash::hash;
use serde::Serialize;
use tracing::{error, instrument, warn};
use uuid::Uuid;
use warp::reply::Response;
use warp::Reply;

pub mod interior_ref_list;
pub mod merchandise_list;
pub mod owner;
pub mod shop;
pub mod transaction;

use super::caches::{CachedResponse, CACHES};
use super::problem::{unauthorized_no_api_key, unauthorized_no_owner};
use super::Environment;

#[instrument(level = "debug", skip(env, api_key))]
pub async fn authenticate(env: &Environment, api_key: Option<Uuid>) -> Result<i32> {
    if let Some(api_key) = api_key {
        CACHES
            .owner_ids_by_api_key
            .get(api_key, || async {
                Ok(
                    sqlx::query!("SELECT id FROM owners WHERE api_key = $1", api_key)
                        .fetch_one(&env.db)
                        .await
                        .map_err(|error| {
                            if let sqlx::Error::RowNotFound = error {
                                return unauthorized_no_owner();
                            }
                            anyhow!(error)
                        })?
                        .id,
                )
            })
            .await
    } else {
        Err(unauthorized_no_api_key())
    }
}

// Similar to `warp::reply::Json`, but stores hash of body content for the ETag header created in `into_response`.
// Also, it does not store a serialize `Result`. Instead it returns the error to the caller immediately in `from_serializable`.
// It's purpose is to avoid serializing the body content twice and to encapsulate ETag logic in one place.
pub struct JsonWithETag {
    body: Vec<u8>,
    etag: String,
}

impl Reply for JsonWithETag {
    fn into_response(self) -> Response {
        let mut res = Response::new(self.body.into());
        res.headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Ok(val) = HeaderValue::from_str(&self.etag) {
            res.headers_mut().insert(ETAG, val);
        } else {
            // This should never happen in practice since etag values should only be hex-encoded strings
            warn!("omitting etag header with invalid ASCII characters")
        }
        res
    }
}

impl JsonWithETag {
    pub fn from_serializable<T: Serialize>(val: &T) -> Result<Self> {
        let bytes = serde_json::to_vec(val).map_err(|err| {
            error!("Failed to serialize database value to JSON: {}", err);
            anyhow!(HttpApiProblem::with_title_and_type_from_status(
                StatusCode::INTERNAL_SERVER_ERROR
            )
            .set_detail(format!(
                "Failed to serialize database value to JSON: {}",
                err
            )))
        })?;
        let etag = format!("{:x}", hash(&bytes));
        Ok(Self { body: bytes, etag })
    }
}

pub fn check_etag(etag: Option<String>, response: CachedResponse) -> CachedResponse {
    if let Some(request_etag) = etag {
        if let Some(response_etag) = response.headers.get("etag") {
            if request_etag == *response_etag {
                return CachedResponse::not_modified(response_etag.clone());
            }
        }
    }
    response
}
