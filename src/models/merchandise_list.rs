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
// support user-defined types (`form_list` Json field).
// See for more info: https://github.com/thallada/rust_sqlx_bug/blob/master/src/main.rs
// This may be fixed in sqlx 0.4.

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Merchandise {
    pub mod_name: String,
    pub local_form_id: i32,
    pub name: String,
    pub quantity: i32,
    pub form_type: i32,
    pub is_food: bool,
    pub price: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MerchandiseList {
    pub id: Option<i32>,
    pub shop_id: i32,
    pub owner_id: Option<i32>,
    pub form_list: Json<Vec<Merchandise>>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, Deserialize)]
pub struct MerchandiseParams {
    pub mod_name: String,
    pub local_form_id: i32,
    pub quantity_delta: i32,
}

#[async_trait]
impl Model for MerchandiseList {
    fn resource_name() -> &'static str {
        "merchandise_list"
    }

    fn pk(&self) -> Option<i32> {
        self.id
    }

    // TODO: this model will probably never need to be accessed through it's ID, should these methods be removed/unimplemented?
    #[instrument(level = "debug", skip(db))]
    async fn get(db: &PgPool, id: i32) -> Result<Self> {
        sqlx::query_as_unchecked!(Self, "SELECT * FROM merchandise_lists WHERE id = $1", id)
            .fetch_one(db)
            .await
            .map_err(Error::new)
    }

    #[instrument(level = "debug", skip(self, db))]
    async fn create(self, db: &PgPool) -> Result<Self> {
        Ok(sqlx::query_as_unchecked!(
            Self,
            "INSERT INTO merchandise_lists
            (shop_id, owner_id, form_list, created_at, updated_at)
            VALUES ($1, $2, $3, now(), now())
            RETURNING *",
            self.shop_id,
            self.owner_id,
            self.form_list,
        )
        .fetch_one(db)
        .await?)
    }

    #[instrument(level = "debug", skip(db))]
    async fn delete(db: &PgPool, owner_id: i32, id: i32) -> Result<u64> {
        let merchandise_list =
            sqlx::query!("SELECT owner_id FROM merchandise_lists WHERE id = $1", id)
                .fetch_one(db)
                .await?;
        if merchandise_list.owner_id == owner_id {
            return Ok(
                sqlx::query!("DELETE FROM merchandise_lists WHERE id = $1", id)
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
                "SELECT * FROM merchandise_lists
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
                "SELECT * FROM merchandise_lists
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
impl UpdateableModel for MerchandiseList {
    #[instrument(level = "debug", skip(self, db))]
    async fn update(self, db: &PgPool, owner_id: i32, id: i32) -> Result<Self> {
        let merchandise_list =
            sqlx::query!("SELECT owner_id FROM merchandise_lists WHERE id = $1", id)
                .fetch_one(db)
                .await?;
        if merchandise_list.owner_id == owner_id {
            Ok(sqlx::query_as_unchecked!(
                Self,
                "UPDATE merchandise_lists SET
                form_list = $2,
                updated_at = now()
                WHERE id = $1
                RETURNING *",
                id,
                self.form_list,
            )
            .fetch_one(db)
            .await?)
        } else {
            return Err(forbidden_permission());
        }
    }
}

impl MerchandiseList {
    #[instrument(level = "debug", skip(db))]
    pub async fn get_by_shop_id(db: &PgPool, shop_id: i32) -> Result<Self> {
        sqlx::query_as_unchecked!(
            Self,
            "SELECT * FROM merchandise_lists
            WHERE shop_id = $1",
            shop_id,
        )
        .fetch_one(db)
        .await
        .map_err(Error::new)
    }

    #[instrument(level = "debug", skip(db))]
    pub async fn update_by_shop_id(self, db: &PgPool, owner_id: i32, shop_id: i32) -> Result<Self> {
        let merchandise_list = sqlx::query!(
            "SELECT owner_id FROM merchandise_lists WHERE shop_id = $1",
            shop_id
        )
        .fetch_one(db)
        .await?;
        if merchandise_list.owner_id == owner_id {
            Ok(sqlx::query_as_unchecked!(
                Self,
                "UPDATE merchandise_lists SET
                form_list = $2,
                updated_at = now()
                WHERE shop_id = $1
                RETURNING *",
                shop_id,
                self.form_list,
            )
            .fetch_one(db)
            .await?)
        } else {
            return Err(forbidden_permission());
        }
    }

    #[instrument(level = "debug", skip(db))]
    pub async fn update_merchandise_quantity(
        db: &PgPool,
        shop_id: i32,
        mod_name: &str,
        local_form_id: i32,
        quantity_delta: i32,
    ) -> Result<Self> {
        Ok(sqlx::query_as_unchecked!(
            Self,
            "UPDATE
                merchandise_lists
            SET
                form_list = CASE
                    WHEN quantity::int + $4 = 0
                        THEN form_list - elem_index::int
                    ELSE jsonb_set(
                        form_list,
                        array[elem_index::text, 'quantity'],
                        to_jsonb(quantity::int + $4),
                        true
                    )
                END
            FROM (
                SELECT
                    pos - 1 as elem_index,
                    elem->>'quantity' as quantity
                FROM
                    merchandise_lists,
                    jsonb_array_elements(form_list) with ordinality arr(elem, pos)
                WHERE
                    shop_id = $1 AND
                    elem->>'mod_name' = $2::text AND
                    elem->>'local_form_id' = $3::text
            ) sub
            WHERE
                shop_id = $1
            RETURNING merchandise_lists.*",
            shop_id,
            mod_name,
            local_form_id,
            quantity_delta,
        )
        .fetch_one(db)
        .await?)
    }
}
