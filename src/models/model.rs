use anyhow::{anyhow, Result};
use async_trait::async_trait;
use sqlx::postgres::PgPool;
use url::Url;

use super::ListParams;

pub trait PostedModel {}

#[async_trait]
pub trait Model
where
    Self: std::marker::Sized,
{
    fn resource_name() -> &'static str;
    fn pk(&self) -> i32;
    fn url(&self, api_url: &Url) -> Result<Url> {
        Ok(api_url.join(&format!("{}s/{}", Self::resource_name(), self.pk()))?)
    }
    async fn get(db: &PgPool, id: i32) -> Result<Self>;
    async fn create(posted: dyn PostedModel, db: &PgPool) -> Result<Self>;
    async fn delete(db: &PgPool, owner_id: i32, id: i32) -> Result<u64>;
    async fn list(db: &PgPool, list_params: &ListParams) -> Result<Vec<Self>>;
}

#[async_trait]
pub trait UpdateableModel
where
    Self: std::marker::Sized,
{
    async fn update(posted: dyn PostedModel, db: &PgPool, owner_id: i32, id: i32) -> Result<Self>;
}
