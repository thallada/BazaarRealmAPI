use anyhow::{Error, Result};
use async_trait::async_trait;
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;
use sqlx::types::Json;
use tracing::instrument;

use super::ListParams;
use super::{Model, UpdateableModel};
use crate::problem::forbidden_permission;

// sqlx queries for this model need to be `query_as_unchecked!` because `query_as!` does not
// support user-defined types (`ref_list` Json field).
// See for more info: https://github.com/thallada/rust_sqlx_bug/blob/master/src/main.rs
// This may be fixed in sqlx 0.4.

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InteriorRef {
    pub base_mod_name: String,
    pub base_local_form_id: i32,
    pub ref_mod_name: Option<String>,
    pub ref_local_form_id: i32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub angle_x: f32,
    pub angle_y: f32,
    pub angle_z: f32,
    pub scale: u16,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InteriorRefList {
    pub id: Option<i32>,
    pub shop_id: i32,
    pub owner_id: Option<i32>,
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

    // TODO: this model will probably never need to be accessed through it's ID, should these methods be removed/unimplemented?
    #[instrument(level = "debug", skip(db))]
    async fn get(db: &PgPool, id: i32) -> Result<Self> {
        sqlx::query_as_unchecked!(Self, "SELECT * FROM interior_ref_lists WHERE id = $1", id)
            .fetch_one(db)
            .await
            .map_err(Error::new)
    }

    #[instrument(level = "debug", skip(self, db))]
    async fn create(self, db: &PgPool) -> Result<Self> {
        // TODO:
        // * Decide if I'll need to make the same changes to merchandise and transactions
        //      - answer depends on how many rows of each I expect to insert in one go
        // * should probably omit ref_list from response
        Ok(sqlx::query_as_unchecked!(
            Self,
            "INSERT INTO interior_ref_lists
            (shop_id, owner_id, ref_list, created_at, updated_at)
            VALUES ($1, $2, $3, now(), now())
            RETURNING *",
            self.shop_id,
            self.owner_id,
            self.ref_list,
        )
        .fetch_one(db)
        .await?)
    }

    #[instrument(level = "debug", skip(db))]
    async fn delete(db: &PgPool, owner_id: i32, id: i32) -> Result<u64> {
        let interior_ref_list =
            sqlx::query!("SELECT owner_id FROM interior_ref_lists WHERE id = $1", id)
                .fetch_one(db)
                .await?;
        if interior_ref_list.owner_id == owner_id {
            return Ok(
                sqlx::query!("DELETE FROM interior_ref_lists WHERE id = $1", id)
                    .execute(db)
                    .await?,
            );
        } else {
            return Err(forbidden_permission());
        }
    }

    #[instrument(level = "debug", skip(db))]
    async fn list(db: &PgPool, list_params: &ListParams) -> Result<Vec<Self>> {
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

#[async_trait]
impl UpdateableModel for InteriorRefList {
    #[instrument(level = "debug", skip(self, db))]
    async fn update(self, db: &PgPool, owner_id: i32, id: i32) -> Result<Self> {
        let interior_ref_list =
            sqlx::query!("SELECT owner_id FROM interior_ref_lists WHERE id = $1", id)
                .fetch_one(db)
                .await?;
        if interior_ref_list.owner_id == owner_id {
            Ok(sqlx::query_as_unchecked!(
                Self,
                "UPDATE interior_ref_lists SET
                ref_list = $2,
                updated_at = now()
                WHERE id = $1
                RETURNING *",
                id,
                self.ref_list,
            )
            .fetch_one(db)
            .await?)
        } else {
            return Err(forbidden_permission());
        }
    }
}

impl InteriorRefList {
    #[instrument(level = "debug", skip(db))]
    pub async fn get_by_shop_id(db: &PgPool, shop_id: i32) -> Result<Self> {
        sqlx::query_as_unchecked!(
            Self,
            "SELECT * FROM interior_ref_lists
            WHERE shop_id = $1",
            shop_id,
        )
        .fetch_one(db)
        .await
        .map_err(Error::new)
    }

    #[instrument(level = "debug", skip(self, db))]
    pub async fn update_by_shop_id(self, db: &PgPool, owner_id: i32, shop_id: i32) -> Result<Self> {
        let interior_ref_list = sqlx::query!(
            "SELECT owner_id FROM interior_ref_lists WHERE shop_id = $1",
            shop_id
        )
        .fetch_one(db)
        .await?;
        if interior_ref_list.owner_id == owner_id {
            Ok(sqlx::query_as_unchecked!(
                Self,
                "UPDATE interior_ref_lists SET
                ref_list = $2,
                updated_at = now()
                WHERE shop_id = $1
                RETURNING *",
                shop_id,
                self.ref_list,
            )
            .fetch_one(db)
            .await?)
        } else {
            return Err(forbidden_permission());
        }
    }
}
