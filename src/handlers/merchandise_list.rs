use anyhow::Result;
use http::StatusCode;
use uuid::Uuid;
use warp::reply::{with_header, with_status};
use warp::{Rejection, Reply};

use crate::models::{ListParams, MerchandiseList};
use crate::problem::reject_anyhow;
use crate::Environment;

use super::{authenticate, check_etag, JsonWithETag};

pub async fn get(id: i32, etag: Option<String>, env: Environment) -> Result<impl Reply, Rejection> {
    let response = env
        .caches
        .merchandise_list
        .get_response(id, || async {
            let merchandise_list = MerchandiseList::get(&env.db, id).await?;
            let reply = JsonWithETag::from_serializable(&merchandise_list)?;
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await?;
    Ok(check_etag(etag, response))
}

pub async fn get_by_shop_id(
    shop_id: i32,
    etag: Option<String>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let response = env
        .caches
        .merchandise_list_by_shop_id
        .get_response(shop_id, || async {
            let merchandise_list = MerchandiseList::get_by_shop_id(&env.db, shop_id).await?;
            let reply = JsonWithETag::from_serializable(&merchandise_list)?;
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
        .list_merchandise_lists
        .get_response(list_params.clone(), || async {
            let merchandise_lists = MerchandiseList::list(&env.db, &list_params).await?;
            let reply = JsonWithETag::from_serializable(&merchandise_lists)?;
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await?;
    Ok(check_etag(etag, response))
}

pub async fn create(
    merchandise_list: MerchandiseList,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    let ref_list_with_owner_id = MerchandiseList {
        owner_id: Some(owner_id),
        ..merchandise_list
    };
    let saved_merchandise_list = ref_list_with_owner_id
        .create(&env.db)
        .await
        .map_err(reject_anyhow)?;
    let url = saved_merchandise_list
        .url(&env.api_url)
        .map_err(reject_anyhow)?;
    let reply = JsonWithETag::from_serializable(&saved_merchandise_list).map_err(reject_anyhow)?;
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    env.caches.list_merchandise_lists.clear().await;
    env.caches
        .merchandise_list_by_shop_id
        .delete_response(saved_merchandise_list.shop_id)
        .await;
    Ok(reply)
}

pub async fn update(
    id: i32,
    merchandise_list: MerchandiseList,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    let merchandise_list_with_id_and_owner_id = if merchandise_list.owner_id.is_some() {
        MerchandiseList {
            id: Some(id),
            ..merchandise_list
        }
    } else {
        MerchandiseList {
            id: Some(id),
            owner_id: Some(owner_id),
            ..merchandise_list
        }
    };
    let updated_merchandise_list = merchandise_list_with_id_and_owner_id
        .update(&env.db, owner_id, id)
        .await
        .map_err(reject_anyhow)?;
    let url = updated_merchandise_list
        .url(&env.api_url)
        .map_err(reject_anyhow)?;
    let reply =
        JsonWithETag::from_serializable(&updated_merchandise_list).map_err(reject_anyhow)?;
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    env.caches.merchandise_list.delete_response(id).await;
    env.caches
        .merchandise_list_by_shop_id
        .delete_response(updated_merchandise_list.shop_id)
        .await;
    env.caches.list_merchandise_lists.clear().await;
    Ok(reply)
}

pub async fn update_by_shop_id(
    shop_id: i32,
    merchandise_list: MerchandiseList,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    let merchandise_list_with_owner_id = MerchandiseList {
        owner_id: Some(owner_id),
        ..merchandise_list
    };
    let updated_merchandise_list = merchandise_list_with_owner_id
        .update_by_shop_id(&env.db, owner_id, shop_id)
        .await
        .map_err(reject_anyhow)?;
    let url = updated_merchandise_list
        .url(&env.api_url)
        .map_err(reject_anyhow)?;
    let reply =
        JsonWithETag::from_serializable(&updated_merchandise_list).map_err(reject_anyhow)?;
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    env.caches
        .merchandise_list
        .delete_response(
            updated_merchandise_list
                .id
                .expect("saved merchandise_list has no id"),
        )
        .await;
    env.caches
        .merchandise_list_by_shop_id
        .delete_response(updated_merchandise_list.shop_id)
        .await;
    env.caches.list_merchandise_lists.clear().await;
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
    env.caches.merchandise_list.delete_response(id).await;
    env.caches
        .merchandise_list_by_shop_id
        .delete_response(merchandise_list.shop_id)
        .await;
    env.caches.list_merchandise_lists.clear().await;
    Ok(StatusCode::NO_CONTENT)
}
