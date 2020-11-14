use anyhow::Result;
use http::StatusCode;
use hyper::body::Bytes;
use mime::Mime;
use uuid::Uuid;
use warp::reply::{with_header, with_status};
use warp::{Rejection, Reply};

use crate::caches::{CachedResponse, CACHES};
use crate::models::{InteriorRefList, ListParams, PostedInteriorRefList};
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
        &CACHES.interior_ref_list_bin,
        &CACHES.interior_ref_list,
    );
    let response = cache
        .get_response(id, || async {
            let interior_ref_list = InteriorRefList::get(&env.db, id).await?;
            let reply: Box<dyn Reply> = match content_type {
                ContentType::Bincode => {
                    Box::new(ETagReply::<Bincode>::from_serializable(&interior_ref_list)?)
                }
                ContentType::Json => {
                    Box::new(ETagReply::<Json>::from_serializable(&interior_ref_list)?)
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
        &CACHES.interior_ref_list_by_shop_id_bin,
        &CACHES.interior_ref_list_by_shop_id,
    );
    let response = cache
        .get_response(shop_id, || async {
            let interior_ref_list = InteriorRefList::get_by_shop_id(&env.db, shop_id).await?;
            let reply: Box<dyn Reply> = match content_type {
                ContentType::Bincode => {
                    Box::new(ETagReply::<Bincode>::from_serializable(&interior_ref_list)?)
                }
                ContentType::Json => {
                    Box::new(ETagReply::<Json>::from_serializable(&interior_ref_list)?)
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
        &CACHES.list_interior_ref_lists_bin,
        &CACHES.list_interior_ref_lists,
    );
    let response = cache
        .get_response(list_params.clone(), || async {
            let interior_ref_lists = InteriorRefList::list(&env.db, &list_params).await?;
            let reply: Box<dyn Reply> = match content_type {
                ContentType::Bincode => Box::new(ETagReply::<Bincode>::from_serializable(
                    &interior_ref_lists,
                )?),
                ContentType::Json => {
                    Box::new(ETagReply::<Json>::from_serializable(&interior_ref_lists)?)
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
        body: mut interior_ref_list,
        content_type,
    } = DeserializedBody::<PostedInteriorRefList>::from_bytes(bytes, content_type)
        .map_err(reject_anyhow)?;
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    interior_ref_list.owner_id = Some(owner_id);
    let saved_interior_ref_list = InteriorRefList::create(interior_ref_list, &env.db)
        .await
        .map_err(reject_anyhow)?;
    let url = saved_interior_ref_list
        .url(&env.api_url)
        .map_err(reject_anyhow)?;
    let reply: Box<dyn Reply> = match content_type {
        ContentType::Bincode => Box::new(
            ETagReply::<Bincode>::from_serializable(&saved_interior_ref_list)
                .map_err(reject_anyhow)?,
        ),
        ContentType::Json => Box::new(
            ETagReply::<Json>::from_serializable(&saved_interior_ref_list)
                .map_err(reject_anyhow)?,
        ),
    };
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    tokio::spawn(async move {
        CACHES.list_interior_ref_lists.clear().await;
        CACHES.list_interior_ref_lists_bin.clear().await;
        CACHES
            .interior_ref_list_by_shop_id
            .delete_response(saved_interior_ref_list.shop_id)
            .await;
        CACHES
            .interior_ref_list_by_shop_id_bin
            .delete_response(saved_interior_ref_list.shop_id)
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
        body: interior_ref_list,
        content_type,
    } = DeserializedBody::<PostedInteriorRefList>::from_bytes(bytes, content_type)
        .map_err(reject_anyhow)?;
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    let updated_interior_ref_list =
        InteriorRefList::update(interior_ref_list, &env.db, owner_id, id)
            .await
            .map_err(reject_anyhow)?;
    let url = updated_interior_ref_list
        .url(&env.api_url)
        .map_err(reject_anyhow)?;
    let reply: Box<dyn Reply> = match content_type {
        ContentType::Bincode => Box::new(
            ETagReply::<Bincode>::from_serializable(&updated_interior_ref_list)
                .map_err(reject_anyhow)?,
        ),
        ContentType::Json => Box::new(
            ETagReply::<Json>::from_serializable(&updated_interior_ref_list)
                .map_err(reject_anyhow)?,
        ),
    };
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    tokio::spawn(async move {
        CACHES.interior_ref_list.delete_response(id).await;
        CACHES.interior_ref_list_bin.delete_response(id).await;
        CACHES
            .interior_ref_list_by_shop_id
            .delete_response(updated_interior_ref_list.shop_id)
            .await;
        CACHES
            .interior_ref_list_by_shop_id_bin
            .delete_response(updated_interior_ref_list.shop_id)
            .await;
        CACHES.list_interior_ref_lists.clear().await;
        CACHES.list_interior_ref_lists_bin.clear().await;
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
        body: interior_ref_list,
        content_type,
    } = DeserializedBody::<PostedInteriorRefList>::from_bytes(bytes, content_type)
        .map_err(reject_anyhow)?;
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    let updated_interior_ref_list =
        InteriorRefList::update_by_shop_id(interior_ref_list, &env.db, owner_id, shop_id)
            .await
            .map_err(reject_anyhow)?;
    let url = updated_interior_ref_list
        .url(&env.api_url)
        .map_err(reject_anyhow)?;
    let reply: Box<dyn Reply> = match content_type {
        ContentType::Bincode => Box::new(
            ETagReply::<Bincode>::from_serializable(&updated_interior_ref_list)
                .map_err(reject_anyhow)?,
        ),
        ContentType::Json => Box::new(
            ETagReply::<Json>::from_serializable(&updated_interior_ref_list)
                .map_err(reject_anyhow)?,
        ),
    };
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    tokio::spawn(async move {
        CACHES
            .interior_ref_list
            .delete_response(updated_interior_ref_list.id)
            .await;
        CACHES
            .interior_ref_list_bin
            .delete_response(updated_interior_ref_list.id)
            .await;
        CACHES
            .interior_ref_list_by_shop_id
            .delete_response(updated_interior_ref_list.shop_id)
            .await;
        CACHES
            .interior_ref_list_by_shop_id_bin
            .delete_response(updated_interior_ref_list.shop_id)
            .await;
        CACHES.list_interior_ref_lists.clear().await;
        CACHES.list_interior_ref_lists_bin.clear().await;
    });
    Ok(reply)
}

pub async fn delete(
    id: i32,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    let interior_ref_list = InteriorRefList::get(&env.db, id)
        .await
        .map_err(reject_anyhow)?;
    InteriorRefList::delete(&env.db, owner_id, id)
        .await
        .map_err(reject_anyhow)?;
    tokio::spawn(async move {
        CACHES.interior_ref_list.delete_response(id).await;
        CACHES.interior_ref_list_bin.delete_response(id).await;
        CACHES
            .interior_ref_list_by_shop_id
            .delete_response(interior_ref_list.shop_id)
            .await;
        CACHES
            .interior_ref_list_by_shop_id_bin
            .delete_response(interior_ref_list.shop_id)
            .await;
        CACHES.list_interior_ref_lists.clear().await;
        CACHES.list_interior_ref_lists_bin.clear().await;
    });
    Ok(StatusCode::NO_CONTENT)
}
