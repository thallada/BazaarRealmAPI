use anyhow::Result;
use async_trait::async_trait;
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::postgres::PgPool;

use super::ListParams;
use super::Model;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Shop {
    pub id: Option<i32>,
    pub name: String,
    pub owner_id: i32,
    pub description: String,
    pub is_not_sell_buy: bool,
    pub sell_buy_list_id: i32,
    pub vendor_id: i32,
    pub vendor_gold: i32,
    pub interior_refs: serde_json::value::Value,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[async_trait]
impl Model for Shop {
    fn resource_name() -> &'static str {
        "shop"
    }

    fn pk(&self) -> Option<i32> {
        self.id
    }

    async fn get(db: &PgPool, id: i32) -> Result<Self> {
        let timer = std::time::Instant::now();
        let result = sqlx::query_as!(Self, "SELECT * FROM shops WHERE id = $1", id)
            .fetch_one(db)
            .await?;
        let elapsed = timer.elapsed();
        debug!("SELECT * FROM shops ... {:.3?}", elapsed);
        Ok(result)
    }

    async fn save(self, db: &PgPool) -> Result<Self> {
        let timer = std::time::Instant::now();
        let result = sqlx::query_as!(
            Self,
            "INSERT INTO shops
            (name, owner_id, description, is_not_sell_buy, sell_buy_list_id, vendor_id,
             vendor_gold, interior_refs, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, now(), now())
            RETURNING *",
            self.name,
            self.owner_id,
            self.description,
            self.is_not_sell_buy,
            self.sell_buy_list_id,
            self.vendor_id,
            self.vendor_gold,
            self.interior_refs,
        )
        .fetch_one(db)
        .await?;
        let elapsed = timer.elapsed();
        debug!("INSERT INTO shops ... {:.3?}", elapsed);
        Ok(result)
    }

    async fn list(db: &PgPool, list_params: ListParams) -> Result<Vec<Self>> {
        let timer = std::time::Instant::now();
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
        let elapsed = timer.elapsed();
        debug!("SELECT * FROM shops ... {:.3?}", elapsed);
        Ok(result)
    }
}
