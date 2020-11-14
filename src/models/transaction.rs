use anyhow::{Error, Result};
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::{Done, Executor, Postgres};
use tracing::instrument;
use url::Url;

use super::ListParams;
use crate::problem::forbidden_permission;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transaction {
    pub id: i32,
    pub shop_id: i32,
    pub owner_id: i32,
    pub mod_name: String,
    pub local_form_id: i32,
    pub name: String,
    pub form_type: i32,
    pub is_food: bool,
    pub price: i32,
    pub is_sell: bool,
    pub quantity: i32,
    pub amount: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PostedTransaction {
    pub shop_id: i32,
    pub owner_id: Option<i32>,
    pub mod_name: String,
    pub local_form_id: i32,
    pub name: String,
    pub form_type: i32,
    pub is_food: bool,
    pub price: i32,
    pub is_sell: bool,
    pub quantity: i32,
    pub amount: i32,
}

impl Transaction {
    pub fn resource_name() -> &'static str {
        "transaction"
    }

    pub fn pk(&self) -> i32 {
        self.id
    }

    pub fn url(&self, api_url: &Url) -> Result<Url> {
        Ok(api_url.join(&format!("{}s/{}", Self::resource_name(), self.pk()))?)
    }

    #[instrument(level = "debug", skip(db))]
    pub async fn get(db: impl Executor<'_, Database = Postgres>, id: i32) -> Result<Self> {
        sqlx::query_as!(Self, "SELECT * FROM transactions WHERE id = $1", id)
            .fetch_one(db)
            .await
            .map_err(Error::new)
    }

    #[instrument(level = "debug", skip(db))]
    pub async fn create(
        transaction: PostedTransaction,
        db: impl Executor<'_, Database = Postgres>,
    ) -> Result<Self> {
        Ok(sqlx::query_as!(
            Self,
            "INSERT INTO transactions
            (shop_id, owner_id, mod_name, local_form_id, name, form_type, is_food, price,
             is_sell, quantity, amount, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, now(), now())
            RETURNING *",
            transaction.shop_id,
            transaction.owner_id,
            transaction.mod_name,
            transaction.local_form_id,
            transaction.name,
            transaction.form_type,
            transaction.is_food,
            transaction.price,
            transaction.is_sell,
            transaction.quantity,
            transaction.amount,
        )
        .fetch_one(db)
        .await?)
    }

    #[instrument(level = "debug", skip(db))]
    pub async fn delete(
        db: impl Executor<'_, Database = Postgres> + Copy,
        owner_id: i32,
        id: i32,
    ) -> Result<u64> {
        let transaction = sqlx::query!("SELECT owner_id FROM transactions WHERE id = $1", id)
            .fetch_one(db)
            .await?;
        if transaction.owner_id == owner_id {
            return Ok(sqlx::query!("DELETE FROM transactions WHERE id = $1", id)
                .execute(db)
                .await?
                .rows_affected());
        } else {
            return Err(forbidden_permission());
        }
    }

    #[instrument(level = "debug", skip(db))]
    pub async fn list(
        db: impl Executor<'_, Database = Postgres>,
        list_params: &ListParams,
    ) -> Result<Vec<Self>> {
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

    #[instrument(level = "debug", skip(db))]
    pub async fn list_by_shop_id(
        db: impl Executor<'_, Database = Postgres>,
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
