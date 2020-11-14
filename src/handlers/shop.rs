use anyhow::{anyhow, Result};
use http::StatusCode;
use hyper::body::Bytes;
use mime::Mime;
use uuid::Uuid;
use warp::reply::{with_header, with_status};
use warp::{Rejection, Reply};

use crate::caches::{CachedResponse, CACHES};
use crate::models::{
    InteriorRefList, ListParams, MerchandiseList, PostedInteriorRefList, PostedMerchandiseList,
    PostedShop, Shop,
};
use crate::problem::reject_anyhow;
use crate::Environment;

use super::{
    authenticate, check_etag, AcceptHeader, Bincode, ContentType, DataReply, DeserializedBody,
    ETagReply, Json, TypedCache,
};

pub async fn get(
    id: i32,
    etag: Option<String>,
    accept: Option<AcceptHeader>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let TypedCache {
        content_type,
        cache,
    } = TypedCache::<i32, CachedResponse>::pick_cache(accept, &CACHES.shop_bin, &CACHES.shop);
    let response = cache
        .get_response(id, || async {
            let shop = Shop::get(&env.db, id).await?;
            let reply: Box<dyn Reply> = match content_type {
                ContentType::Bincode => Box::new(ETagReply::<Bincode>::from_serializable(&shop)?),
                ContentType::Json => Box::new(ETagReply::<Json>::from_serializable(&shop)?),
            };
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await?;
    Ok(check_etag(etag, response))
}

pub async fn list(
    list_params: ListParams,
    etag: Option<String>,
    accept: Option<AcceptHeader>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let TypedCache {
        content_type,
        cache,
    } = TypedCache::<ListParams, CachedResponse>::pick_cache(
        accept,
        &CACHES.list_shops_bin,
        &CACHES.list_shops,
    );
    let response = cache
        .get_response(list_params.clone(), || async {
            let shops = Shop::list(&env.db, &list_params).await?;
            let reply: Box<dyn Reply> = match content_type {
                ContentType::Bincode => Box::new(ETagReply::<Bincode>::from_serializable(&shops)?),
                ContentType::Json => Box::new(ETagReply::<Json>::from_serializable(&shops)?),
            };
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await?;
    Ok(check_etag(etag, response))
}

pub async fn create(
    bytes: Bytes,
    api_key: Option<Uuid>,
    content_type: Option<Mime>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let DeserializedBody {
        body: mut shop,
        content_type,
    } = DeserializedBody::<PostedShop>::from_bytes(bytes, content_type).map_err(reject_anyhow)?;
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    shop.owner_id = Some(owner_id);
    let mut tx = env
        .db
        .begin()
        .await
        .map_err(|error| reject_anyhow(anyhow!(error)))?;
    let saved_shop = Shop::create(shop, &mut tx).await.map_err(reject_anyhow)?;

    // also save empty interior_ref_list and merchandise_list rows
    let interior_ref_list = PostedInteriorRefList {
        shop_id: saved_shop.id,
        owner_id: Some(owner_id),
        ref_list: sqlx::types::Json::default(),
    };
    InteriorRefList::create(interior_ref_list, &mut tx)
        .await
        .map_err(reject_anyhow)?;
    let merchandise_list = PostedMerchandiseList {
        shop_id: saved_shop.id,
        owner_id: Some(owner_id),
        form_list: sqlx::types::Json::default(),
    };
    MerchandiseList::create(merchandise_list, &mut tx)
        .await
        .map_err(reject_anyhow)?;
    tx.commit()
        .await
        .map_err(|error| reject_anyhow(anyhow!(error)))?;

    let url = saved_shop.url(&env.api_url).map_err(reject_anyhow)?;
    let reply: Box<dyn Reply> = match content_type {
        ContentType::Bincode => {
            Box::new(ETagReply::<Bincode>::from_serializable(&saved_shop).map_err(reject_anyhow)?)
        }
        ContentType::Json => {
            Box::new(ETagReply::<Json>::from_serializable(&saved_shop).map_err(reject_anyhow)?)
        }
    };
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    tokio::spawn(async move {
        CACHES.list_shops.clear().await;
        CACHES.list_shops_bin.clear().await;
    });
    Ok(reply)
}

pub async fn update(
    id: i32,
    bytes: Bytes,
    api_key: Option<Uuid>,
    content_type: Option<Mime>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let DeserializedBody {
        body: mut shop,
        content_type,
    } = DeserializedBody::<PostedShop>::from_bytes(bytes, content_type).map_err(reject_anyhow)?;
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    shop.owner_id = match shop.owner_id {
        // allows an owner to transfer ownership of shop to another owner
        Some(posted_owner_id) => Some(posted_owner_id),
        None => Some(owner_id),
    };
    let updated_shop = Shop::update(shop, &env.db, owner_id, id)
        .await
        .map_err(reject_anyhow)?;
    let url = updated_shop.url(&env.api_url).map_err(reject_anyhow)?;
    let reply: Box<dyn Reply> = match content_type {
        ContentType::Bincode => {
            Box::new(ETagReply::<Bincode>::from_serializable(&updated_shop).map_err(reject_anyhow)?)
        }
        ContentType::Json => {
            Box::new(ETagReply::<Json>::from_serializable(&updated_shop).map_err(reject_anyhow)?)
        }
    };
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    tokio::spawn(async move {
        CACHES.shop.delete_response(id).await;
        CACHES.shop_bin.delete_response(id).await;
        CACHES.list_shops.clear().await;
        CACHES.list_shops_bin.clear().await;
    });
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
