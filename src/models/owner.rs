use anyhow::{anyhow, Error, Result};
use chrono::prelude::*;
use ipnetwork::IpNetwork;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::instrument;
use url::Url;
use uuid::Uuid;

use super::ListParams;
use crate::problem::forbidden_permission;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Owner {
    pub id: Option<i32>,
    pub name: String,
    #[serde(skip_serializing)]
    pub api_key: Option<Uuid>,
    #[serde(skip_serializing)]
    pub ip_address: Option<IpNetwork>,
    pub mod_version: i32,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

impl Owner {
    pub fn resource_name() -> &'static str {
        "owner"
    }

    pub fn pk(&self) -> Option<i32> {
        self.id
    }

    pub fn url(&self, api_url: &Url) -> Result<Url> {
        if let Some(pk) = self.pk() {
            Ok(api_url.join(&format!("{}s/{}", Self::resource_name(), pk))?)
        } else {
            Err(anyhow!(
                "Cannot get URL for {} with no primary key",
                Self::resource_name()
            ))
        }
    }

    #[instrument(level = "debug", skip(db))]
    pub async fn get(db: &PgPool, id: i32) -> Result<Self> {
        sqlx::query_as!(Self, "SELECT * FROM owners WHERE id = $1", id)
            .fetch_one(db)
            .await
            .map_err(Error::new)
    }

    #[instrument(level = "debug", skip(self, db))]
    pub async fn create(self, db: &PgPool) -> Result<Self> {
        Ok(sqlx::query_as!(
            Self,
            "INSERT INTO owners
                (name, api_key, ip_address, mod_version, created_at, updated_at)
                VALUES ($1, $2, $3, $4, now(), now())
                RETURNING *",
            self.name,
            self.api_key,
            self.ip_address,
            self.mod_version,
        )
        .fetch_one(db)
        .await?)
    }

    #[instrument(level = "debug", skip(db))]
    pub async fn delete(db: &PgPool, owner_id: i32, id: i32) -> Result<u64> {
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
    pub async fn list(db: &PgPool, list_params: &ListParams) -> Result<Vec<Self>> {
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

    #[instrument(level = "debug", skip(self, db))]
    pub async fn update(self, db: &PgPool, owner_id: i32, id: i32) -> Result<Self> {
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
                self.name,
                self.mod_version,
            )
            .fetch_one(db)
            .await?)
        } else {
            return Err(forbidden_permission());
        }
    }
}
