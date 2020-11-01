use anyhow::{anyhow, Result};
use async_trait::async_trait;
use sqlx::postgres::PgPool;
use url::Url;

use super::ListParams;

// TODO: I stopped using this because I needed to accept a transaction instead of a &PgPool for these methods on certain models.
// It would be nice to find a way to impl this trait for all my models so I don't have to keep redoing the `url` function on
// each. But, maybe I'm trying to use Traits in an OOP way and that's bad, idk.
//
// @NyxCode on discord: "on 0.4, you can use impl Executor<'_, Database = Postgres>. I use it everywhere, and it works for
// &PgPool, &mut PgConnection and &mut Transaction"
//
// I attempted to use `impl Executor<Database = Postgres>` in 0.3.5 but it created a recursive type error :(
#[async_trait]
pub trait Model
where
    Self: std::marker::Sized,
{
    fn resource_name() -> &'static str;
    fn pk(&self) -> Option<i32>;
    fn url(&self, api_url: &Url) -> Result<Url> {
        if let Some(pk) = self.pk() {
            Ok(api_url.join(&format!("{}s/{}", Self::resource_name(), pk))?)
        } else {
            Err(anyhow!(
                "Cannot get URL for {} with no primary key",
                Self::resource_name()
            ))
        }
    }
    async fn get(db: &PgPool, id: i32) -> Result<Self>;
    async fn create(self, db: &PgPool) -> Result<Self>;
    async fn delete(db: &PgPool, owner_id: i32, id: i32) -> Result<u64>;
    async fn list(db: &PgPool, list_params: &ListParams) -> Result<Vec<Self>>;
}

#[async_trait]
pub trait UpdateableModel
where
    Self: std::marker::Sized,
{
    async fn update(self, db: &PgPool, owner_id: i32, id: i32) -> Result<Self>;
}
