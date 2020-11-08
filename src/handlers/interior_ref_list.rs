use anyhow::Result;
use http::StatusCode;
use mime::Mime;
use uuid::Uuid;
use warp::reply::{with_header, with_status};
use warp::{Rejection, Reply};

use crate::caches::{Cache, CachedResponse, CACHES};
use crate::models::{InteriorRefList, ListParams};
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
                let interior_ref_list = InteriorRefList::get(&env.db, id).await?;
                let reply = T::from_serializable(&interior_ref_list)?;
                let reply = with_status(reply, StatusCode::OK);
                Ok(reply)
            })
            .await?;
        Ok(Box::new(check_etag(etag, response)))
    }

    match accept {
        Some(accept) if accept.accepts_bincode() => {
            get::<ETagReply<Bincode>>(id, etag, env, &CACHES.interior_ref_list_bin).await
        }
        _ => get::<ETagReply<Json>>(id, etag, env, &CACHES.interior_ref_list).await,
    }
}

pub async fn get_by_shop_id(
    shop_id: i32,
    etag: Option<String>,
    accept: Option<AcceptHeader>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    async fn get<T: DataReply>(
        shop_id: i32,
        etag: Option<String>,
        env: Environment,
        cache: &'static Cache<i32, CachedResponse>,
    ) -> Result<Box<dyn Reply>, Rejection> {
        let response = cache
            .get_response(shop_id, || async {
                let interior_ref_list = InteriorRefList::get_by_shop_id(&env.db, shop_id).await?;
                let reply = T::from_serializable(&interior_ref_list)?;
                let reply = with_status(reply, StatusCode::OK);
                Ok(reply)
            })
            .await?;
        Ok(Box::new(check_etag(etag, response)))
    }

    match accept {
        Some(accept) if accept.accepts_bincode() => {
            get::<ETagReply<Bincode>>(shop_id, etag, env, &CACHES.interior_ref_list_by_shop_id_bin)
                .await
        }
        _ => get::<ETagReply<Json>>(shop_id, etag, env, &CACHES.interior_ref_list_by_shop_id).await,
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
                let interior_ref_lists = InteriorRefList::list(&env.db, &list_params).await?;
                let reply = T::from_serializable(&interior_ref_lists)?;
                let reply = with_status(reply, StatusCode::OK);
                Ok(reply)
            })
            .await?;

        Ok(Box::new(check_etag(etag, response)))
    }

    match accept {
        Some(accept) if accept.accepts_bincode() => {
            get::<ETagReply<Bincode>>(list_params, etag, env, &CACHES.list_interior_ref_lists_bin)
                .await
        }
        _ => get::<ETagReply<Json>>(list_params, etag, env, &CACHES.list_interior_ref_lists).await,
    }
}

pub async fn create(
    interior_ref_list: InteriorRefList,
    api_key: Option<Uuid>,
    content_type: Option<Mime>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    async fn create<'a, T: DataReply + 'a>(
        interior_ref_list: InteriorRefList,
        api_key: Option<Uuid>,
        env: Environment,
    ) -> Result<Box<dyn Reply + 'a>, Rejection> {
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
        let reply = ETagReply::<Json>::from_serializable(&saved_interior_ref_list)
            .map_err(reject_anyhow)?;
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
        Ok(Box::new(reply))
    }

    match content_type {
        Some(content_type) if content_type == mime::APPLICATION_OCTET_STREAM => {
            create::<ETagReply<Bincode>>(interior_ref_list, api_key, env).await
        }
        _ => create::<ETagReply<Json>>(interior_ref_list, api_key, env).await,
    }
}

pub async fn update(
    id: i32,
    interior_ref_list: InteriorRefList,
    api_key: Option<Uuid>,
    content_type: Option<Mime>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    async fn update<'a, T: DataReply + 'a>(
        id: i32,
        interior_ref_list: InteriorRefList,
        api_key: Option<Uuid>,
        env: Environment,
    ) -> Result<Box<dyn Reply + 'a>, Rejection> {
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
        let reply = T::from_serializable(&updated_interior_ref_list).map_err(reject_anyhow)?;
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
        Ok(Box::new(reply))
    }

    match content_type {
        Some(content_type) if content_type == mime::APPLICATION_OCTET_STREAM => {
            update::<ETagReply<Bincode>>(id, interior_ref_list, api_key, env).await
        }
        _ => update::<ETagReply<Json>>(id, interior_ref_list, api_key, env).await,
    }
}

pub async fn update_by_shop_id(
    shop_id: i32,
    interior_ref_list: InteriorRefList,
    api_key: Option<Uuid>,
    content_type: Option<Mime>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    async fn update<'a, T: DataReply + 'a>(
        shop_id: i32,
        interior_ref_list: InteriorRefList,
        api_key: Option<Uuid>,
        env: Environment,
    ) -> Result<Box<dyn Reply + 'a>, Rejection> {
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
        let reply = T::from_serializable(&updated_interior_ref_list).map_err(reject_anyhow)?;
        let reply = with_header(reply, "Location", url.as_str());
        let reply = with_status(reply, StatusCode::CREATED);
        tokio::spawn(async move {
            let id = updated_interior_ref_list
                .id
                .expect("saved interior_ref_list has no id");
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
        Ok(Box::new(reply))
    }

    match content_type {
        Some(content_type) if content_type == mime::APPLICATION_OCTET_STREAM => {
            update::<ETagReply<Bincode>>(shop_id, interior_ref_list, api_key, env).await
        }
        _ => update::<ETagReply<Json>>(shop_id, interior_ref_list, api_key, env).await,
    }
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
