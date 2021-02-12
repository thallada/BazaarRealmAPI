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
    pub gold: i32,
    pub shop_type: String,
    pub vendor_keywords: Vec<String>,
    pub vendor_keywords_exclude: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PostedShop {
    pub name: String,
    pub owner_id: Option<i32>,
    pub description: Option<String>,
    pub gold: Option<i32>,
    pub shop_type: Option<String>,
    pub vendor_keywords: Option<Vec<String>>,
    pub vendor_keywords_exclude: Option<bool>,
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
            (name, owner_id, description, gold, shop_type, vendor_keywords,
             vendor_keywords_exclude, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, now(), now())
            RETURNING *",
            shop.name,
            shop.owner_id,
            shop.description,
            shop.gold.unwrap_or(0),
            shop.shop_type.unwrap_or("general_store".to_string()),
            &shop
                .vendor_keywords
                .unwrap_or_else(|| vec!["VendorItemKey".to_string(), "VendorNoSale".to_string()]),
            shop.vendor_keywords_exclude.unwrap_or(true),
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
                gold = $5,
                shop_type = $6,
                vendor_keywords = $7,
                vendor_keywords_exclude = $8,
                updated_at = now()
                WHERE id = $1
                RETURNING *",
                id,
                shop.name,
                shop.owner_id,
                shop.description,
                shop.gold,
                shop.shop_type,
                &shop.vendor_keywords.unwrap_or_else(|| vec![]),
                shop.vendor_keywords_exclude,
            )
            .fetch_one(db)
            .await?)
        } else {
            return Err(forbidden_permission());
        }
    }

    #[instrument(level = "debug", skip(db))]
    pub async fn accepts_keywords(
        db: impl Executor<'_, Database = Postgres>,
        id: i32,
        keywords: &[String],
    ) -> Result<bool> {
        // Macro not available, see: https://github.com/launchbadge/sqlx/issues/428
        Ok(sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM shops
                WHERE id = $1
                    AND ((
                        vendor_keywords_exclude = true AND
                        NOT vendor_keywords && $2
                    ) OR (
                        vendor_keywords_exclude = false AND
                        vendor_keywords && $2
                    ))
            )",
        )
        .bind(id)
        .bind(keywords)
        .fetch_one(db)
        .await?)
    }

    #[instrument(level = "debug", skip(db))]
    pub async fn update_gold(
        db: impl Executor<'_, Database = Postgres>,
        id: i32,
        gold_delta: i32,
    ) -> Result<()> {
        sqlx::query!(
            "UPDATE shops SET
                gold = gold + $2
            WHERE id = $1",
            id,
            gold_delta,
        )
        .execute(db)
        .await?;
        Ok(())
    }
}
