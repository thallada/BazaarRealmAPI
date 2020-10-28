use anyhow::{Error, Result};
use async_trait::async_trait;
use chrono::prelude::*;
use ipnetwork::IpNetwork;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;
use tracing::instrument;
use uuid::Uuid;

use super::ListParams;
use super::{Model, PostedModel, UpdateableModel};
use crate::problem::forbidden_permission;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Owner {
    pub id: i32,
    pub name: String,
    #[serde(skip_serializing)]
    pub api_key: Uuid,
    #[serde(skip_serializing)]
    pub ip_address: Option<IpNetwork>,
    pub mod_version: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PostedOwner {
    pub name: String,
    #[serde(skip_serializing)]
    pub api_key: Option<Uuid>,
    #[serde(skip_serializing)]
    pub ip_address: Option<IpNetwork>,
    pub mod_version: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl PostedModel for PostedOwner {}

#[async_trait]
impl Model for Owner {
    fn resource_name() -> &'static str {
        "owner"
    }

    fn pk(&self) -> i32 {
        self.id
    }

    #[instrument(level = "debug", skip(db))]
    async fn get(db: &PgPool, id: i32) -> Result<Self> {
        sqlx::query_as!(Self, "SELECT * FROM owners WHERE id = $1", id)
            .fetch_one(db)
            .await
            .map_err(Error::new)
    }

    #[instrument(level = "debug", skip(posted, db))]
    async fn create(posted: PostedOwner, db: &PgPool) -> Result<Self> {
        Ok(sqlx::query_as!(
            Self,
            "INSERT INTO owners
                (name, api_key, ip_address, mod_version, created_at, updated_at)
                VALUES ($1, $2, $3, $4, now(), now())
                RETURNING *",
            posted.name,
            posted.api_key,
            posted.ip_address,
            posted.mod_version,
        )
        .fetch_one(db)
        .await?)
    }

    #[instrument(level = "debug", skip(db))]
    async fn delete(db: &PgPool, owner_id: i32, id: i32) -> Result<u64> {
        let owner = sqlx::query!("SELECT id FROM owners WHERE id = $1", id)
            .fetch_one(db)
            .await?;
        if owner.id == owner_id {
            Ok(sqlx::query!("DELETE FROM owners WHERE id = $1", id)
                .execute(db)
                .await?)
        } else {
            return Err(forbidden_permission());
        }
    }

    #[instrument(level = "debug", skip(db))]
    async fn list(db: &PgPool, list_params: &ListParams) -> Result<Vec<Self>> {
        let result = if let Some(order_by) = list_params.get_order_by() {
            sqlx::query_as!(
                Self,
                "SELECT * FROM owners
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
                "SELECT * FROM owners
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
impl UpdateableModel for Owner {
    #[instrument(level = "debug", skip(posted, db))]
    async fn update(posted: PostedOwner, db: &PgPool, owner_id: i32, id: i32) -> Result<Self> {
        let owner = sqlx::query!("SELECT id FROM owners WHERE id = $1", id)
            .fetch_one(db)
            .await?;
        if owner.id == owner_id {
            Ok(sqlx::query_as!(
                Self,
                "UPDATE owners SET
                name = $2,
                mod_version = $3,
                updated_at = now()
                WHERE id = $1
                RETURNING *",
                id,
                posted.name,
                posted.mod_version,
            )
            .fetch_one(db)
            .await?)
        } else {
            return Err(forbidden_permission());
        }
    }
}
