use anyhow::Result;
use http::StatusCode;
use mime::Mime;
use uuid::Uuid;
use warp::reply::{with_header, with_status};
use warp::{Rejection, Reply};

use crate::caches::{Cache, CachedResponse, CACHES};
use crate::models::{InteriorRefList, ListParams, MerchandiseList, Shop};
use crate::problem::reject_anyhow;
use crate::Environment;

use super::{authenticate, check_etag, AcceptHeader, Bincode, DataReply, ETagReply, Json};

pub async fn get(
    id: i32,
    etag: Option<String>,
    accept: Option<AcceptHeader>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    async fn get<T: DataReply>(
        id: i32,
        etag: Option<String>,
        env: Environment,
        cache: &'static Cache<i32, CachedResponse>,
    ) -> Result<Box<dyn Reply>, Rejection> {
        let response = cache
            .get_response(id, || async {
                let shop = Shop::get(&env.db, id).await?;
                let reply = T::from_serializable(&shop)?;
                let reply = with_status(reply, StatusCode::OK);
                Ok(reply)
            })
            .await?;
        Ok(Box::new(check_etag(etag, response)))
    }

    match accept {
        Some(accept) if accept.accepts_bincode() => {
            get::<ETagReply<Bincode>>(id, etag, env, &CACHES.shop_bin).await
        }
        _ => get::<ETagReply<Json>>(id, etag, env, &CACHES.shop).await,
    }
}

pub async fn list(
    list_params: ListParams,
    etag: Option<String>,
    accept: Option<AcceptHeader>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    async fn get<T: DataReply>(
        list_params: ListParams,
        etag: Option<String>,
        env: Environment,
        cache: &'static Cache<ListParams, CachedResponse>,
    ) -> Result<Box<dyn Reply>, Rejection> {
        let response = cache
            .get_response(list_params.clone(), || async {
                let shops = Shop::list(&env.db, &list_params).await?;
                let reply = T::from_serializable(&shops)?;
                let reply = with_status(reply, StatusCode::OK);
                Ok(reply)
            })
            .await?;
        Ok(Box::new(check_etag(etag, response)))
    }

    match accept {
        Some(accept) if accept.accepts_bincode() => {
            get::<ETagReply<Bincode>>(list_params, etag, env, &CACHES.list_shops_bin).await
        }
        _ => get::<ETagReply<Json>>(list_params, etag, env, &CACHES.list_shops).await,
    }
}

pub async fn create(
    shop: Shop,
    api_key: Option<Uuid>,
    content_type: Option<Mime>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    async fn create<'a, T: DataReply + 'a>(
        shop: Shop,
        api_key: Option<Uuid>,
        env: Environment,
    ) -> Result<Box<dyn Reply + 'a>, Rejection> {
        let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
        let shop_with_owner_id = Shop {
            owner_id: Some(owner_id),
            ..shop
        };
        let saved_shop = shop_with_owner_id
            .create(&env.db)
            .await
            .map_err(reject_anyhow)?;

        // also save empty interior_ref_list and merchandise_list rows
        // TODO: do this in a transaction with shop.create
        if let Some(shop_id) = saved_shop.id {
            let interior_ref_list = InteriorRefList {
                id: None,
                shop_id,
                owner_id: Some(owner_id),
                ref_list: sqlx::types::Json::default(),
                created_at: None,
                updated_at: None,
            };
            interior_ref_list
                .create(&env.db)
                .await
                .map_err(reject_anyhow)?;
            let merchandise_list = MerchandiseList {
                id: None,
                shop_id,
                owner_id: Some(owner_id),
                form_list: sqlx::types::Json::default(),
                created_at: None,
                updated_at: None,
            };
            merchandise_list
                .create(&env.db)
                .await
                .map_err(reject_anyhow)?;
        }

        let url = saved_shop.url(&env.api_url).map_err(reject_anyhow)?;
        let reply = T::from_serializable(&saved_shop).map_err(reject_anyhow)?;
        let reply = with_header(reply, "Location", url.as_str());
        let reply = with_status(reply, StatusCode::CREATED);
        tokio::spawn(async move {
            CACHES.list_shops.clear().await;
            CACHES.list_shops_bin.clear().await;
        });
        Ok(Box::new(reply))
    }

    match content_type {
        Some(content_type) if content_type == mime::APPLICATION_OCTET_STREAM => {
            create::<ETagReply<Bincode>>(shop, api_key, env).await
        }
        _ => create::<ETagReply<Json>>(shop, api_key, env).await,
    }
}

pub async fn update(
    id: i32,
    shop: Shop,
    api_key: Option<Uuid>,
    content_type: Option<Mime>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    async fn update<'a, T: DataReply + 'a>(
        id: i32,
        shop: Shop,
        api_key: Option<Uuid>,
        env: Environment,
    ) -> Result<Box<dyn Reply + 'a>, Rejection> {
        let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
        let shop_with_id_and_owner_id = if shop.owner_id.is_some() {
            // allows an owner to transfer ownership of shop to another owner
            Shop {
                id: Some(id),
                ..shop
            }
        } else {
            Shop {
                id: Some(id),
                owner_id: Some(owner_id),
                ..shop
            }
        };
        let updated_shop = shop_with_id_and_owner_id
            .update(&env.db, owner_id, id)
            .await
            .map_err(reject_anyhow)?;
        let url = updated_shop.url(&env.api_url).map_err(reject_anyhow)?;
        let reply = T::from_serializable(&updated_shop).map_err(reject_anyhow)?;
        let reply = with_header(reply, "Location", url.as_str());
        let reply = with_status(reply, StatusCode::CREATED);
        tokio::spawn(async move {
            CACHES.shop.delete_response(id).await;
            CACHES.shop_bin.delete_response(id).await;
            CACHES.list_shops.clear().await;
            CACHES.list_shops_bin.clear().await;
        });
        Ok(Box::new(reply))
    }

    match content_type {
        Some(content_type) if content_type == mime::APPLICATION_OCTET_STREAM => {
            update::<ETagReply<Bincode>>(id, shop, api_key, env).await
        }
        _ => update::<ETagReply<Json>>(id, shop, api_key, env).await,
    }
}

pub async fn delete(
    id: i32,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    Shop::delete(&env.db, owner_id, id)
        .await
        .map_err(reject_anyhow)?;
    tokio::spawn(async move {
        CACHES.shop.delete_response(id).await;
        CACHES.shop_bin.delete_response(id).await;
        CACHES.list_shops.clear().await;
        CACHES.list_shops_bin.clear().await;
        CACHES
            .interior_ref_list_by_shop_id
            .delete_response(id)
            .await;
        CACHES
            .interior_ref_list_by_shop_id_bin
            .delete_response(id)
            .await;
        CACHES.merchandise_list_by_shop_id.delete_response(id).await;
        CACHES
            .merchandise_list_by_shop_id_bin
            .delete_response(id)
            .await;
    });
    Ok(StatusCode::NO_CONTENT)
}
