use anyhow::{Error, Result};
use async_trait::async_trait;
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;
use tracing::instrument;

use super::ListParams;
use super::Model;
use crate::problem::forbidden_permission;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transaction {
    pub id: Option<i32>,
    pub shop_id: i32,
    pub owner_id: Option<i32>,
    pub mod_name: String,
    pub local_form_id: i32,
    pub is_sell: bool,
    pub quantity: i32,
    pub amount: i32,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[async_trait]
impl Model for Transaction {
    fn resource_name() -> &'static str {
        "transaction"
    }

    fn pk(&self) -> Option<i32> {
        self.id
    }

    #[instrument(level = "debug", skip(db))]
    async fn get(db: &PgPool, id: i32) -> Result<Self> {
        sqlx::query_as!(Self, "SELECT * FROM transactions WHERE id = $1", id)
            .fetch_one(db)
            .await
            .map_err(Error::new)
    }

    #[instrument(level = "debug", skip(db))]
    async fn create(self, db: &PgPool) -> Result<Self> {
        Ok(sqlx::query_as!(
            Self,
            "INSERT INTO transactions
            (shop_id, owner_id, mod_name, local_form_id, is_sell, quantity, amount, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, now(), now())
            RETURNING *",
            self.shop_id,
            self.owner_id,
            self.mod_name,
            self.local_form_id,
            self.is_sell,
            self.quantity,
            self.amount,
        )
        .fetch_one(db)
        .await?)
    }

    #[instrument(level = "debug", skip(db))]
    async fn delete(db: &PgPool, owner_id: i32, id: i32) -> Result<u64> {
        let transaction = sqlx::query!("SELECT owner_id FROM transactions WHERE id = $1", id)
            .fetch_one(db)
            .await?;
        if transaction.owner_id == owner_id {
            return Ok(sqlx::query!("DELETE FROM transactions WHERE id = $1", id)
                .execute(db)
                .await?);
        } else {
            return Err(forbidden_permission());
        }
    }

    #[instrument(level = "debug", skip(db))]
    async fn list(db: &PgPool, list_params: &ListParams) -> Result<Vec<Self>> {
        let result = if let Some(order_by) = list_params.get_order_by() {
            sqlx::query_as!(
                Self,
                "SELECT * FROM transactions
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
            sqlx::query_as!(
                Self,
                "SELECT * FROM transactions
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

impl Transaction {
    #[instrument(level = "debug", skip(db))]
    pub async fn list_by_shop_id(
        db: &PgPool,
        shop_id: i32,
        list_params: &ListParams,
    ) -> Result<Vec<Self>> {
        let result = if let Some(order_by) = list_params.get_order_by() {
            sqlx::query_as!(
                Self,
                "SELECT * FROM transactions
                WHERE shop_id = $1
                ORDER BY $2
                LIMIT $3
                OFFSET $4",
                shop_id,
                order_by,
                list_params.limit.unwrap_or(10),
                list_params.offset.unwrap_or(0),
            )
            .fetch_all(db)
            .await?
        } else {
            sqlx::query_as!(
                Self,
                "SELECT * FROM transactions
                WHERE shop_id = $1
                LIMIT $2
                OFFSET $3",
                shop_id,
                list_params.limit.unwrap_or(10),
                list_params.offset.unwrap_or(0),
            )
            .fetch_all(db)
            .await?
        };
        Ok(result)
    }
}
