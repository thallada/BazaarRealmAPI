use anyhow::{Error, Result};
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::{Done, Executor, Postgres};
use tracing::instrument;
use url::Url;

use super::ListParams;
use crate::problem::forbidden_permission;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Shop {
    pub id: i32,
    pub name: String,
    pub owner_id: i32,
    pub description: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PostedShop {
    pub name: String,
    pub owner_id: Option<i32>,
    pub description: Option<String>,
}

impl Shop {
    pub fn resource_name() -> &'static str {
        "shop"
    }

    pub fn pk(&self) -> i32 {
        self.id
    }

    pub fn url(&self, api_url: &Url) -> Result<Url> {
        Ok(api_url.join(&format!("{}s/{}", Self::resource_name(), self.pk()))?)
    }

    #[instrument(level = "debug", skip(db))]
    pub async fn get(db: impl Executor<'_, Database = Postgres>, id: i32) -> Result<Self> {
        sqlx::query_as!(Self, "SELECT * FROM shops WHERE id = $1", id)
            .fetch_one(db)
            .await
            .map_err(Error::new)
    }

    #[instrument(level = "debug", skip(shop, db))]
    pub async fn create(
        shop: PostedShop,
        db: impl Executor<'_, Database = Postgres>,
    ) -> Result<Self> {
        Ok(sqlx::query_as!(
            Self,
            "INSERT INTO shops
            (name, owner_id, description, created_at, updated_at)
            VALUES ($1, $2, $3, now(), now())
            RETURNING *",
            shop.name,
            shop.owner_id,
            shop.description,
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
        let shop = sqlx::query!("SELECT owner_id FROM shops WHERE id = $1", id)
            .fetch_one(db)
            .await?;
        if shop.owner_id == owner_id {
            return Ok(sqlx::query!("DELETE FROM shops WHERE shops.id = $1", id)
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
                "SELECT * FROM shops
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
                "SELECT * FROM shops
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

    #[instrument(level = "debug", skip(shop, db))]
    pub async fn update(
        shop: PostedShop,
        db: impl Executor<'_, Database = Postgres> + Copy,
        owner_id: i32,
        id: i32,
    ) -> Result<Self> {
        let existing_shop = sqlx::query!("SELECT owner_id FROM shops WHERE id = $1", id)
            .fetch_one(db)
            .await?;
        if existing_shop.owner_id == owner_id {
            Ok(sqlx::query_as!(
                Self,
                "UPDATE shops SET
                name = $2,
                owner_id = $3,
                description = $4,
                updated_at = now()
                WHERE id = $1
                RETURNING *",
                id,
                shop.name,
                shop.owner_id,
                shop.description,
            )
            .fetch_one(db)
            .await?)
        } else {
            return Err(forbidden_permission());
        }
    }
}
