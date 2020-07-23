use anyhow::Result;
use async_trait::async_trait;
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::postgres::PgPool;

use super::ListParams;
use super::Model;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InteriorRef {
    pub id: Option<i32>,
    pub shop_id: i32,
    pub references: serde_json::value::Value,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[async_trait]
impl Model for InteriorRef {
    fn resource_name() -> &'static str {
        "interior_ref"
    }

    fn pk(&self) -> Option<i32> {
        self.id
    }

    async fn get(db: &PgPool, id: i32) -> Result<Self> {
        let timer = std::time::Instant::now();
        let result = sqlx::query_as!(Self, "SELECT * FROM interior_refs WHERE id = $1", id)
            .fetch_one(db)
            .await?;
        let elapsed = timer.elapsed();
        debug!("SELECT * FROM interior_refs ... {:.3?}", elapsed);
        Ok(result)
    }

    async fn save(self, db: &PgPool) -> Result<Self> {
        let timer = std::time::Instant::now();
        // TODO:
        // * Actually save the references list to the jsonb column
        // * Decide if I'll need to make the same changes to merchandise and transactions
        //      - answer depends on how many rows of each I expect to insert in one go
        let result = sqlx::query_as!(
            Self,
            "INSERT INTO interior_refs
            (shop_id, created_at, updated_at)
            VALUES ($1, now(), now())
            RETURNING *",
            self.shop_id,
        )
        .fetch_one(db)
        .await?;
        let elapsed = timer.elapsed();
        debug!("INSERT INTO interior_refs ... {:.3?}", elapsed);
        Ok(result)
    }

    async fn list(db: &PgPool, list_params: ListParams) -> Result<Vec<Self>> {
        let timer = std::time::Instant::now();
        let result = if let Some(order_by) = list_params.get_order_by() {
            sqlx::query_as!(
                Self,
                "SELECT * FROM interior_refs
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
                "SELECT * FROM interior_refs
                LIMIT $1
                OFFSET $2",
                list_params.limit.unwrap_or(10),
                list_params.offset.unwrap_or(0),
            )
            .fetch_all(db)
            .await?
        };
        let elapsed = timer.elapsed();
        debug!("SELECT * FROM interior_refs ... {:.3?}", elapsed);
        Ok(result)
    }
}
