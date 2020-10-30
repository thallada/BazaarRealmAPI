use anyhow::{anyhow, Result};
use tracing::instrument;
use uuid::Uuid;

pub mod interior_ref_list;
pub mod merchandise_list;
pub mod owner;
pub mod shop;
pub mod transaction;

use super::problem::{unauthorized_no_api_key, unauthorized_no_owner};
use super::Environment;

#[instrument(level = "debug", skip(env, api_key))]
pub async fn authenticate(env: &Environment, api_key: Option<Uuid>) -> Result<i32> {
    if let Some(api_key) = api_key {
        env.caches
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
