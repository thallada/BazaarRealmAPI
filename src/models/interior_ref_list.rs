use anyhow::Result;
use async_trait::async_trait;
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;
use sqlx::types::Json;
use tracing::instrument;

use super::ListParams;
use super::Model;

// sqlx queries for this model need to be `query_as_unchecked!` because `query_as!` does not
// support user-defined types (`ref_list` Json field).
// See for more info: https://github.com/thallada/rust_sqlx_bug/blob/master/src/main.rs
// This may be fixed in sqlx 0.4.

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InteriorRef {
    pub mod_name: String,
    pub local_form_id: i32,
    pub position_x: f64,
    pub position_y: f64,
    pub position_z: f64,
    pub angle_x: f64,
    pub angle_y: f64,
    pub angle_z: f64,
    pub scale: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InteriorRefList {
    pub id: Option<i32>,
    pub shop_id: i32,
    pub ref_list: Json<Vec<InteriorRef>>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[async_trait]
impl Model for InteriorRefList {
    fn resource_name() -> &'static str {
        "interior_ref_list"
    }

    fn pk(&self) -> Option<i32> {
        self.id
    }

    #[instrument(level = "debug", skip(db))]
    async fn get(db: &PgPool, id: i32) -> Result<Self> {
        Ok(
            sqlx::query_as_unchecked!(Self, "SELECT * FROM interior_ref_lists WHERE id = $1", id)
                .fetch_one(db)
                .await?,
        )
    }

    #[instrument(level = "debug", skip(self, db))]
    async fn save(self, db: &PgPool) -> Result<Self> {
        // TODO:
        // * Decide if I'll need to make the same changes to merchandise and transactions
        //      - answer depends on how many rows of each I expect to insert in one go
        // * should probably omit ref_list from response
        Ok(sqlx::query_as_unchecked!(
            Self,
            "INSERT INTO interior_ref_lists
            (shop_id, ref_list, created_at, updated_at)
            VALUES ($1, $2, now(), now())
            RETURNING *",
            self.shop_id,
            self.ref_list,
        )
        .fetch_one(db)
        .await?)
    }

    #[instrument(level = "debug", skip(db))]
    async fn delete(db: &PgPool, id: i32) -> Result<u64> {
        Ok(
            sqlx::query!("DELETE FROM interior_ref_lists WHERE id = $1", id)
                .execute(db)
                .await?,
        )
    }

    #[instrument(level = "debug", skip(db))]
    async fn list(db: &PgPool, list_params: ListParams) -> Result<Vec<Self>> {
        let result = if let Some(order_by) = list_params.get_order_by() {
            sqlx::query_as_unchecked!(
                Self,
                "SELECT * FROM interior_ref_lists
                ORDER BY $1
                LIMIT $2
                OFFSET $3",
                order_by,
                list_params.limit.unwrap_or(10),
                list_params.offset.unwrap_or(0),
            )
            .fetch_all(db)
            .await?
        } else {
            sqlx::query_as_unchecked!(
                Self,
                "SELECT * FROM interior_ref_lists
                LIMIT $1
                OFFSET $2",
                list_params.limit.unwrap_or(10),
                list_params.offset.unwrap_or(0),
            )
            .fetch_all(db)
            .await?
        };
        Ok(result)
    }
}
