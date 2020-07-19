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
    pub mod_name: String,
    pub local_form_id: i32,
    pub position_x: f64,
    pub position_y: f64,
    pub position_z: f64,
    pub angle_x: f64,
    pub angle_y: f64,
    pub angle_z: f64,
    pub scale: f64,
    pub created_at: Option<NaiveDateTime>,
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
        let result = sqlx::query_as!(
            Self,
            "INSERT INTO interior_refs
            (shop_id, mod_name, local_form_id, position_x, position_y, position_z, angle_x,
            angle_y, angle_z, scale, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, now())
            RETURNING *",
            self.shop_id,
            self.mod_name,
            self.local_form_id,
            self.position_x,
            self.position_y,
            self.position_z,
            self.angle_x,
            self.angle_y,
            self.angle_z,
            self.scale,
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

    // TODO: figure out a way bulk insert in a single query
    // see: https://github.com/launchbadge/sqlx/issues/294
    async fn bulk_save(db: &PgPool, interior_refs: Vec<Self>) -> Result<()> {
        let timer = std::time::Instant::now();
        // Testing whether setting a jsonb column with an array of 200 items is faster than
        // inserting 200 rows. Answer: it is a hell of a lot faster!
        // TODO:
        // 1. remove interior_refs column from shops
        // 2. replace all columns in interior_refs table with single `refs` jsonb column and
        //    shops_id foreign_key
        // 3. This function will now create the row in that table
        // 4. Decide if I'll need to make the same changes to merchandise and transactions
        //      - answer depends on how many rows of each I expect to insert in one go
        sqlx::query!(
            "UPDATE shops SET interior_refs = $1::jsonb",
            serde_json::to_value(&interior_refs)?,
        )
        .execute(db)
        .await?;
        // let mut transaction = db.begin().await?;
        // for interior_ref in interior_refs {
        // sqlx::query!(
        // "INSERT INTO interior_refs
        // (shop_id, mod_name, local_form_id, position_x, position_y, position_z, angle_x,
        // angle_y, angle_z, scale, created_at)
        // VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, now())",
        // interior_ref.shop_id,
        // interior_ref.mod_name,
        // interior_ref.local_form_id,
        // interior_ref.position_x,
        // interior_ref.position_y,
        // interior_ref.position_z,
        // interior_ref.angle_x,
        // interior_ref.angle_y,
        // interior_ref.angle_z,
        // interior_ref.scale,
        // )
        // .execute(&mut transaction)
        // .await?;
        // }
        // transaction.commit().await?;
        let elapsed = timer.elapsed();
        debug!("INSERT INTO interior_refs ... {:.3?}", elapsed);
        Ok(())
    }
}
