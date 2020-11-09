use anyhow::Result;
use http::StatusCode;
use mime::Mime;
use uuid::Uuid;
use warp::reply::{with_header, with_status};
use warp::{Rejection, Reply};

use crate::caches::CACHES;
use crate::models::{ListParams, MerchandiseList, PostedMerchandiseList, UnsavedMerchandiseList};
use crate::problem::reject_anyhow;
use crate::Environment;

use super::{
    authenticate, check_etag, AcceptHeader, Bincode, ContentType, DataReply, ETagReply, Json,
};

pub async fn get(
    id: i32,
    etag: Option<String>,
    accept: Option<AcceptHeader>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let (content_type, cache) = match accept {
        Some(accept) if accept.accepts_bincode() => {
            (ContentType::Bincode, &CACHES.merchandise_list_bin)
        }
        _ => (ContentType::Json, &CACHES.merchandise_list),
    };
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
    let (content_type, cache) = match accept {
        Some(accept) if accept.accepts_bincode() => {
            (ContentType::Bincode, &CACHES.merchandise_list_bin)
        }
        _ => (ContentType::Json, &CACHES.merchandise_list),
    };
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
    let (content_type, cache) = match accept {
        Some(accept) if accept.accepts_bincode() => {
            (ContentType::Bincode, &CACHES.list_merchandise_lists_bin)
        }
        _ => (ContentType::Json, &CACHES.list_merchandise_lists),
    };
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
    merchandise_list: PostedMerchandiseList,
    api_key: Option<Uuid>,
    content_type: Option<Mime>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let content_type = match content_type {
        Some(content_type) if content_type == mime::APPLICATION_OCTET_STREAM => {
            ContentType::Bincode
        }
        _ => ContentType::Json,
    };
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    let unsaved_merchandise_list = UnsavedMerchandiseList {
        owner_id,
        shop_id: merchandise_list.shop_id,
        form_list: merchandise_list.form_list,
    };
    let saved_merchandise_list = MerchandiseList::create(unsaved_merchandise_list, &env.db)
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
    merchandise_list: PostedMerchandiseList,
    api_key: Option<Uuid>,
    content_type: Option<Mime>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let content_type = match content_type {
        Some(content_type) if content_type == mime::APPLICATION_OCTET_STREAM => {
            ContentType::Bincode
        }
        _ => ContentType::Json,
    };
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
    merchandise_list: PostedMerchandiseList,
    api_key: Option<Uuid>,
    content_type: Option<Mime>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let content_type = match content_type {
        Some(content_type) if content_type == mime::APPLICATION_OCTET_STREAM => {
            ContentType::Bincode
        }
        _ => ContentType::Json,
    };
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
