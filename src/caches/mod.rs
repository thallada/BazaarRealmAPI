use std::fmt::Debug;
use uuid::Uuid;

use crate::models::ListParams;

mod cache;
mod cached_response;

pub use cache::Cache;
pub use cached_response::CachedResponse;

lazy_static! {
    pub static ref CACHES: Caches = Caches::initialize();
}

#[derive(Debug, Clone)]
pub struct Caches {
    pub owner_ids_by_api_key: Cache<Uuid, i32>,
    pub shop: Cache<i32, CachedResponse>,
    pub owner: Cache<i32, CachedResponse>,
    pub interior_ref_list: Cache<i32, CachedResponse>,
    pub merchandise_list: Cache<i32, CachedResponse>,
    pub transaction: Cache<i32, CachedResponse>,
    pub list_shops: Cache<ListParams, CachedResponse>,
    pub list_owners: Cache<ListParams, CachedResponse>,
    pub list_interior_ref_lists: Cache<ListParams, CachedResponse>,
    pub list_merchandise_lists: Cache<ListParams, CachedResponse>,
    pub list_transactions: Cache<ListParams, CachedResponse>,
    pub list_transactions_by_shop_id: Cache<(i32, ListParams), CachedResponse>,
    pub interior_ref_list_by_shop_id: Cache<i32, CachedResponse>,
    pub merchandise_list_by_shop_id: Cache<i32, CachedResponse>,
}

impl Caches {
    pub fn initialize() -> Self {
        Caches {
            owner_ids_by_api_key: Cache::new("owner_ids_by_api_key", 100).log_keys(false),
            shop: Cache::new("shop", 100),
            owner: Cache::new("owner", 100),
            interior_ref_list: Cache::new("interior_ref_list", 100),
            merchandise_list: Cache::new("merchandise_list", 100),
            transaction: Cache::new("transaction", 100),
            list_shops: Cache::new("list_shops", 100),
            list_owners: Cache::new("list_owners", 100),
            list_interior_ref_lists: Cache::new("list_interior_ref_lists", 100),
            list_merchandise_lists: Cache::new("list_merchandise_lists", 100),
            list_transactions: Cache::new("list_transaction", 100),
            list_transactions_by_shop_id: Cache::new("list_transaction_by_shop_id", 100),
            interior_ref_list_by_shop_id: Cache::new("interior_ref_list_by_shop_id", 100),
            merchandise_list_by_shop_id: Cache::new("merchandise_list_by_shop_id", 100),
        }
    }
}
