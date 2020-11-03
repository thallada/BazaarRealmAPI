use anyhow::Result;
use http::StatusCode;
use uuid::Uuid;
use warp::reply::{json, with_header, with_status};
use warp::{Rejection, Reply};

use crate::models::{InteriorRefList, ListParams};
use crate::problem::reject_anyhow;
use crate::Environment;

use super::{authenticate, check_etag, JsonWithETag};

pub async fn get(id: i32, etag: Option<String>, env: Environment) -> Result<impl Reply, Rejection> {
    let response = env
        .caches
        .interior_ref_list
        .get_response(id, || async {
            let interior_ref_list = InteriorRefList::get(&env.db, id).await?;
            let reply = JsonWithETag::from_serializable(&interior_ref_list)?;
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
        .interior_ref_list_by_shop_id
        .get_response(shop_id, || async {
            let interior_ref_list = InteriorRefList::get_by_shop_id(&env.db, shop_id).await?;
            let reply = JsonWithETag::from_serializable(&interior_ref_list)?;
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
        .list_interior_ref_lists
        .get_response(list_params.clone(), || async {
            let interior_ref_lists = InteriorRefList::list(&env.db, &list_params).await?;
            let reply = JsonWithETag::from_serializable(&interior_ref_lists)?;
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await?;
    Ok(check_etag(etag, response))
}

pub async fn create(
    interior_ref_list: InteriorRefList,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    let ref_list_with_owner_id = InteriorRefList {
        owner_id: Some(owner_id),
        ..interior_ref_list
    };
    let saved_interior_ref_list = ref_list_with_owner_id
        .create(&env.db)
        .await
        .map_err(reject_anyhow)?;
    let url = saved_interior_ref_list
        .url(&env.api_url)
        .map_err(reject_anyhow)?;
    let reply = json(&saved_interior_ref_list);
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    env.caches.list_interior_ref_lists.clear().await;
    env.caches
        .interior_ref_list_by_shop_id
        .delete_response(saved_interior_ref_list.shop_id)
        .await;
    Ok(reply)
}

pub async fn update(
    id: i32,
    interior_ref_list: InteriorRefList,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    let interior_ref_list_with_id_and_owner_id = if interior_ref_list.owner_id.is_some() {
        InteriorRefList {
            id: Some(id),
            ..interior_ref_list
        }
    } else {
        InteriorRefList {
            id: Some(id),
            owner_id: Some(owner_id),
            ..interior_ref_list
        }
    };
    let updated_interior_ref_list = interior_ref_list_with_id_and_owner_id
        .update(&env.db, owner_id, id)
        .await
        .map_err(reject_anyhow)?;
    let url = updated_interior_ref_list
        .url(&env.api_url)
        .map_err(reject_anyhow)?;
    let reply = json(&updated_interior_ref_list);
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    env.caches.interior_ref_list.delete_response(id).await;
    env.caches
        .interior_ref_list_by_shop_id
        .delete_response(updated_interior_ref_list.shop_id)
        .await;
    env.caches.list_interior_ref_lists.clear().await;
    Ok(reply)
}

pub async fn update_by_shop_id(
    shop_id: i32,
    interior_ref_list: InteriorRefList,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    let interior_ref_list_with_owner_id = InteriorRefList {
        owner_id: Some(owner_id),
        ..interior_ref_list
    };
    let updated_interior_ref_list = interior_ref_list_with_owner_id
        .update_by_shop_id(&env.db, owner_id, shop_id)
        .await
        .map_err(reject_anyhow)?;
    let url = updated_interior_ref_list
        .url(&env.api_url)
        .map_err(reject_anyhow)?;
    let reply = json(&updated_interior_ref_list);
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    env.caches
        .interior_ref_list
        .delete_response(
            updated_interior_ref_list
                .id
                .expect("saved interior_ref_list has no id"),
        )
        .await;
    env.caches
        .interior_ref_list_by_shop_id
        .delete_response(updated_interior_ref_list.shop_id)
        .await;
    env.caches.list_interior_ref_lists.clear().await;
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
    env.caches.interior_ref_list.delete_response(id).await;
    env.caches.list_interior_ref_lists.clear().await;
    env.caches
        .interior_ref_list_by_shop_id
        .delete_response(interior_ref_list.shop_id)
        .await;
    Ok(StatusCode::NO_CONTENT)
}
