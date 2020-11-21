use anyhow::{Error, Result};
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use sqlx::{Done, Executor, Postgres};
use tracing::instrument;
use url::Url;

use super::ListParams;
use crate::problem::forbidden_permission;

#[derive(sqlx::FromRow, Debug, Serialize, Deserialize, Clone)]
pub struct InteriorRef {
    pub base_mod_name: String,
    pub base_local_form_id: u32,
    pub ref_mod_name: Option<String>,
    pub ref_local_form_id: u32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub angle_x: f32,
    pub angle_y: f32,
    pub angle_z: f32,
    pub scale: u16,
}

#[derive(sqlx::FromRow, Debug, Serialize, Deserialize, Clone)]
pub struct Shelf {
    pub shelf_type: u32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub angle_x: f32,
    pub angle_y: f32,
    pub angle_z: f32,
    pub scale: u16,
    pub page: u32,
    pub filter_form_type: Option<u32>,
    pub filter_is_food: bool,
    pub search: Option<String>,
    pub sort_on: Option<String>,
    pub sort_asc: bool,
}

#[derive(sqlx::FromRow, Debug, Serialize, Deserialize, Clone)]
pub struct InteriorRefList {
    pub id: i32,
    pub shop_id: i32,
    pub owner_id: i32,
    pub ref_list: Json<Vec<InteriorRef>>,
    pub shelves: Json<Vec<Shelf>>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PostedInteriorRefList {
    pub shop_id: i32,
    pub owner_id: Option<i32>,
    pub ref_list: Json<Vec<InteriorRef>>,
    pub shelves: Json<Vec<Shelf>>,
}

impl InteriorRefList {
    pub fn resource_name() -> &'static str {
        "interior_ref_list"
    }

    pub fn pk(&self) -> i32 {
        self.id
    }

    pub fn url(&self, api_url: &Url) -> Result<Url> {
        Ok(api_url.join(&format!("{}s/{}", Self::resource_name(), self.pk()))?)
    }

    // TODO: this model will probably never need to be accessed through it's ID, should these methods be removed/unimplemented?
    #[instrument(level = "debug", skip(db))]
    pub async fn get(db: impl Executor<'_, Database = Postgres>, id: i32) -> Result<Self> {
        sqlx::query_as!(
            Self,
            r#"SELECT id, shop_id, owner_id, created_at, updated_at,
                   ref_list as "ref_list: Json<Vec<InteriorRef>>",
                   shelves as "shelves: Json<Vec<Shelf>>"
               FROM interior_ref_lists WHERE id = $1"#,
            id
        )
        .fetch_one(db)
        .await
        .map_err(Error::new)
    }

    #[instrument(level = "debug", skip(interior_ref_list, db))]
    pub async fn create(
        interior_ref_list: PostedInteriorRefList,
        db: impl Executor<'_, Database = Postgres>,
    ) -> Result<Self> {
        Ok(sqlx::query_as!(
            Self,
            r#"INSERT INTO interior_ref_lists
                (shop_id, owner_id, ref_list, shelves, created_at, updated_at)
            VALUES ($1, $2, $3, $4, now(), now())
            RETURNING id, shop_id, owner_id, created_at, updated_at,
                ref_list as "ref_list: Json<Vec<InteriorRef>>",
                shelves as "shelves: Json<Vec<Shelf>>""#,
            interior_ref_list.shop_id,
            interior_ref_list.owner_id,
            serde_json::json!(interior_ref_list.ref_list),
            serde_json::json!(interior_ref_list.shelves),
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
        let interior_ref_list =
            sqlx::query!("SELECT owner_id FROM interior_ref_lists WHERE id = $1", id)
                .fetch_one(db)
                .await?;
        if interior_ref_list.owner_id == owner_id {
            return Ok(
                sqlx::query!("DELETE FROM interior_ref_lists WHERE id = $1", id)
                    .execute(db)
                    .await?
                    .rows_affected(),
            );
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
                r#"SELECT id, shop_id, owner_id, created_at, updated_at,
                    ref_list as "ref_list: Json<Vec<InteriorRef>>",
                    shelves as "shelves: Json<Vec<Shelf>>" FROM interior_ref_lists
                ORDER BY $1
                LIMIT $2
                OFFSET $3"#,
                order_by,
                list_params.limit.unwrap_or(10),
                list_params.offset.unwrap_or(0),
            )
            .fetch_all(db)
            .await?
        } else {
            sqlx::query_as!(
                Self,
                r#"SELECT id, shop_id, owner_id, created_at, updated_at,
                    ref_list as "ref_list: Json<Vec<InteriorRef>>",
                    shelves as "shelves: Json<Vec<Shelf>>" FROM interior_ref_lists
                LIMIT $1
                OFFSET $2"#,
                list_params.limit.unwrap_or(10),
                list_params.offset.unwrap_or(0),
            )
            .fetch_all(db)
            .await?
        };
        Ok(result)
    }

    #[instrument(level = "debug", skip(interior_ref_list, db))]
    pub async fn update(
        interior_ref_list: PostedInteriorRefList,
        db: impl Executor<'_, Database = Postgres> + Copy,
        owner_id: i32,
        id: i32,
    ) -> Result<Self> {
        let existing_interior_ref_list =
            sqlx::query!("SELECT owner_id FROM interior_ref_lists WHERE id = $1", id)
                .fetch_one(db)
                .await?;
        if existing_interior_ref_list.owner_id == owner_id {
            Ok(sqlx::query_as!(
                Self,
                r#"UPDATE interior_ref_lists SET
                ref_list = $2,
                shelves = $3,
                updated_at = now()
                WHERE id = $1
                RETURNING id, shop_id, owner_id, created_at, updated_at,
                    ref_list as "ref_list: Json<Vec<InteriorRef>>",
                    shelves as "shelves: Json<Vec<Shelf>>""#,
                id,
                serde_json::json!(interior_ref_list.ref_list),
                serde_json::json!(interior_ref_list.shelves),
            )
            .fetch_one(db)
            .await?)
        } else {
            return Err(forbidden_permission());
        }
    }

    #[instrument(level = "debug", skip(db))]
    pub async fn get_by_shop_id(
        db: impl Executor<'_, Database = Postgres>,
        shop_id: i32,
    ) -> Result<Self> {
        sqlx::query_as!(
            Self,
            r#"SELECT id, shop_id, owner_id, created_at, updated_at,
                ref_list as "ref_list: Json<Vec<InteriorRef>>",
                shelves as "shelves: Json<Vec<Shelf>>" FROM interior_ref_lists
            WHERE shop_id = $1"#,
            shop_id,
        )
        .fetch_one(db)
        .await
        .map_err(Error::new)
    }

    #[instrument(level = "debug", skip(interior_ref_list, db))]
    pub async fn update_by_shop_id(
        interior_ref_list: PostedInteriorRefList,
        db: impl Executor<'_, Database = Postgres> + Copy,
        owner_id: i32,
        shop_id: i32,
    ) -> Result<Self> {
        let existing_interior_ref_list = sqlx::query!(
            "SELECT owner_id FROM interior_ref_lists WHERE shop_id = $1",
            shop_id
        )
        .fetch_one(db)
        .await?;
        if existing_interior_ref_list.owner_id == owner_id {
            Ok(sqlx::query_as!(
                Self,
                r#"UPDATE interior_ref_lists SET
                ref_list = $2,
                shelves = $3,
                updated_at = now()
                WHERE shop_id = $1
                RETURNING id, shop_id, owner_id, created_at, updated_at,
                    ref_list as "ref_list: Json<Vec<InteriorRef>>",
                    shelves as "shelves: Json<Vec<Shelf>>""#,
                shop_id,
                serde_json::json!(interior_ref_list.ref_list),
                serde_json::json!(interior_ref_list.shelves),
            )
            .fetch_one(db)
            .await?)
        } else {
            return Err(forbidden_permission());
        }
    }
}
