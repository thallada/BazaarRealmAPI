use anyhow::Result;
use http::StatusCode;
use sqlx::types::Json;
use uuid::Uuid;
use warp::reply::{json, with_header, with_status};
use warp::{Rejection, Reply};

use crate::models::{InteriorRefList, ListParams, MerchandiseList, Shop};
use crate::problem::reject_anyhow;
use crate::Environment;

use super::{authenticate, check_etag, JsonWithETag};

pub async fn get(id: i32, etag: Option<String>, env: Environment) -> Result<impl Reply, Rejection> {
    let response = env
        .caches
        .shop
        .get_response(id, || async {
            let shop = Shop::get(&env.db, id).await?;
            let reply = JsonWithETag::from_serializable(&shop)?;
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await?;
    Ok(check_etag(etag, response))
}

pub async fn list(
    list_params: ListParams,
    etag: Option<String>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let response = env
        .caches
        .list_shops
        .get_response(list_params.clone(), || async {
            let shops = Shop::list(&env.db, &list_params).await?;
            let reply = JsonWithETag::from_serializable(&shops)?;
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await?;
    Ok(check_etag(etag, response))
}

pub async fn create(
    shop: Shop,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
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
    if let Some(shop_id) = saved_shop.id {
        let interior_ref_list = InteriorRefList {
            id: None,
            shop_id,
            owner_id: Some(owner_id),
            ref_list: Json::default(),
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
            form_list: Json::default(),
            created_at: None,
            updated_at: None,
        };
        merchandise_list
            .create(&env.db)
            .await
            .map_err(reject_anyhow)?;
    }

    let url = saved_shop.url(&env.api_url).map_err(reject_anyhow)?;
    let reply = json(&saved_shop);
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    env.caches.list_shops.clear().await;
    Ok(reply)
}

pub async fn update(
    id: i32,
    shop: Shop,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
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
    let reply = json(&updated_shop);
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    env.caches.shop.delete_response(id).await;
    env.caches.list_shops.clear().await;
    Ok(reply)
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
    env.caches.shop.delete_response(id).await;
    env.caches.list_shops.clear().await;
    env.caches
        .interior_ref_list_by_shop_id
        .delete_response(id)
        .await;
    env.caches
        .merchandise_list_by_shop_id
        .delete_response(id)
        .await;
    Ok(StatusCode::NO_CONTENT)
}
