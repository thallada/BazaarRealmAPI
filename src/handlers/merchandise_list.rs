use anyhow::Result;
use http::StatusCode;
use hyper::body::Bytes;
use mime::Mime;
use uuid::Uuid;
use warp::reply::{with_header, with_status};
use warp::{Rejection, Reply};

use crate::caches::{CachedResponse, CACHES};
use crate::models::{ListParams, MerchandiseList, PostedMerchandiseList};
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
    } = TypedCache::<i32, CachedResponse>::pick_cache(
        accept,
        &CACHES.merchandise_list_bin,
        &CACHES.merchandise_list,
    );
    let response = cache
        .get_response(id, || async {
            let merchandise_list = MerchandiseList::get(&env.db, id).await?;
            let reply: Box<dyn Reply> = match content_type {
                ContentType::Bincode => {
                    Box::new(ETagReply::<Bincode>::from_serializable(&merchandise_list)?)
                }
                ContentType::Json => {
                    Box::new(ETagReply::<Json>::from_serializable(&merchandise_list)?)
                }
            };
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await?;
    Ok(check_etag(etag, response))
}

pub async fn get_by_shop_id(
    shop_id: i32,
    etag: Option<String>,
    accept: Option<AcceptHeader>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let TypedCache {
        content_type,
        cache,
    } = TypedCache::<i32, CachedResponse>::pick_cache(
        accept,
        &CACHES.merchandise_list_by_shop_id_bin,
        &CACHES.merchandise_list_by_shop_id,
    );
    let response = cache
        .get_response(shop_id, || async {
            let merchandise_list = MerchandiseList::get_by_shop_id(&env.db, shop_id).await?;
            let reply: Box<dyn Reply> = match content_type {
                ContentType::Bincode => {
                    Box::new(ETagReply::<Bincode>::from_serializable(&merchandise_list)?)
                }
                ContentType::Json => {
                    Box::new(ETagReply::<Json>::from_serializable(&merchandise_list)?)
                }
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
        &CACHES.list_merchandise_lists_bin,
        &CACHES.list_merchandise_lists,
    );
    let response = cache
        .get_response(list_params.clone(), || async {
            let merchandise_lists = MerchandiseList::list(&env.db, &list_params).await?;
            let reply: Box<dyn Reply> = match content_type {
                ContentType::Bincode => {
                    Box::new(ETagReply::<Bincode>::from_serializable(&merchandise_lists)?)
                }
                ContentType::Json => {
                    Box::new(ETagReply::<Json>::from_serializable(&merchandise_lists)?)
                }
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
        body: mut merchandise_list,
        content_type,
    } = DeserializedBody::<PostedMerchandiseList>::from_bytes(bytes, content_type)
        .map_err(reject_anyhow)?;
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    merchandise_list.owner_id = Some(owner_id);
    let saved_merchandise_list = MerchandiseList::create(merchandise_list, &env.db)
        .await
        .map_err(reject_anyhow)?;
    let url = saved_merchandise_list
        .url(&env.api_url)
        .map_err(reject_anyhow)?;
    let reply: Box<dyn Reply> = match content_type {
        ContentType::Bincode => Box::new(
            ETagReply::<Bincode>::from_serializable(&saved_merchandise_list)
                .map_err(reject_anyhow)?,
        ),
        ContentType::Json => Box::new(
            ETagReply::<Json>::from_serializable(&saved_merchandise_list).map_err(reject_anyhow)?,
        ),
    };
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    tokio::spawn(async move {
        CACHES.list_merchandise_lists.clear().await;
        CACHES.list_merchandise_lists_bin.clear().await;
        CACHES
            .merchandise_list_by_shop_id
            .delete_response(saved_merchandise_list.shop_id)
            .await;
        CACHES
            .merchandise_list_by_shop_id_bin
            .delete_response(saved_merchandise_list.shop_id)
            .await;
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
        body: merchandise_list,
        content_type,
    } = DeserializedBody::<PostedMerchandiseList>::from_bytes(bytes, content_type)
        .map_err(reject_anyhow)?;
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    let updated_merchandise_list = MerchandiseList::update(merchandise_list, &env.db, owner_id, id)
        .await
        .map_err(reject_anyhow)?;
    let url = updated_merchandise_list
        .url(&env.api_url)
        .map_err(reject_anyhow)?;
    let reply: Box<dyn Reply> = match content_type {
        ContentType::Bincode => Box::new(
            ETagReply::<Bincode>::from_serializable(&updated_merchandise_list)
                .map_err(reject_anyhow)?,
        ),
        ContentType::Json => Box::new(
            ETagReply::<Json>::from_serializable(&updated_merchandise_list)
                .map_err(reject_anyhow)?,
        ),
    };
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    tokio::spawn(async move {
        CACHES.merchandise_list.delete_response(id).await;
        CACHES.merchandise_list_bin.delete_response(id).await;
        CACHES
            .merchandise_list_by_shop_id
            .delete_response(updated_merchandise_list.shop_id)
            .await;
        CACHES
            .merchandise_list_by_shop_id_bin
            .delete_response(updated_merchandise_list.shop_id)
            .await;
        CACHES.list_merchandise_lists.clear().await;
        CACHES.list_merchandise_lists_bin.clear().await;
    });
    Ok(reply)
}

pub async fn update_by_shop_id(
    shop_id: i32,
    bytes: Bytes,
    api_key: Option<Uuid>,
    content_type: Option<Mime>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let DeserializedBody {
        body: merchandise_list,
        content_type,
    } = DeserializedBody::<PostedMerchandiseList>::from_bytes(bytes, content_type)
        .map_err(reject_anyhow)?;
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    let updated_merchandise_list =
        MerchandiseList::update_by_shop_id(merchandise_list, &env.db, owner_id, shop_id)
            .await
            .map_err(reject_anyhow)?;
    let url = updated_merchandise_list
        .url(&env.api_url)
        .map_err(reject_anyhow)?;
    let reply: Box<dyn Reply> = match content_type {
        ContentType::Bincode => Box::new(
            ETagReply::<Bincode>::from_serializable(&updated_merchandise_list)
                .map_err(reject_anyhow)?,
        ),
        ContentType::Json => Box::new(
            ETagReply::<Json>::from_serializable(&updated_merchandise_list)
                .map_err(reject_anyhow)?,
        ),
    };
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    tokio::spawn(async move {
        CACHES
            .merchandise_list
            .delete_response(updated_merchandise_list.id)
            .await;
        CACHES
            .merchandise_list_bin
            .delete_response(updated_merchandise_list.id)
            .await;
        CACHES
            .merchandise_list_by_shop_id
            .delete_response(updated_merchandise_list.shop_id)
            .await;
        CACHES
            .merchandise_list_by_shop_id_bin
            .delete_response(updated_merchandise_list.shop_id)
            .await;
        CACHES.list_merchandise_lists.clear().await;
        CACHES.list_merchandise_lists_bin.clear().await;
    });
    Ok(reply)
}

pub async fn delete(
    id: i32,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    let merchandise_list = MerchandiseList::get(&env.db, id)
        .await
        .map_err(reject_anyhow)?;
    MerchandiseList::delete(&env.db, owner_id, id)
        .await
        .map_err(reject_anyhow)?;
    tokio::spawn(async move {
        CACHES.merchandise_list.delete_response(id).await;
        CACHES.merchandise_list_bin.delete_response(id).await;
        CACHES
            .merchandise_list_by_shop_id
            .delete_response(merchandise_list.shop_id)
            .await;
        CACHES
            .merchandise_list_by_shop_id_bin
            .delete_response(merchandise_list.shop_id)
            .await;
        CACHES.list_merchandise_lists.clear().await;
        CACHES.list_merchandise_lists_bin.clear().await;
    });
    Ok(StatusCode::NO_CONTENT)
}
