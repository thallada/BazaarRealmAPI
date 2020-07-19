use anyhow::{anyhow, Result};
use async_trait::async_trait;
use sqlx::postgres::PgPool;
use url::Url;

use super::ListParams;

#[async_trait]
pub trait Model
where
    Self: std::marker::Sized,
{
    fn resource_name() -> &'static str;
    fn pk(&self) -> Option<i32>;
    fn url(&self, api_url: &Url) -> Result<Url> {
        if let Some(pk) = self.pk() {
            Ok(api_url.join(&format!("/{}s/{}", Self::resource_name(), pk))?)
        } else {
            Err(anyhow!(
                "Cannot get URL for {} with no primary key",
                Self::resource_name()
            ))
        }
    }
    async fn get(db: &PgPool, id: i32) -> Result<Self>;
    async fn save(self, db: &PgPool) -> Result<Self>;
    async fn list(db: &PgPool, list_params: ListParams) -> Result<Vec<Self>>;
    async fn bulk_save(_db: &PgPool, _models: Vec<Self>) -> Result<()> {
        unimplemented!()
    }
}
