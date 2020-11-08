use anyhow::Result;
use http::StatusCode;
use mime::Mime;
use uuid::Uuid;
use warp::reply::{with_header, with_status};
use warp::{Rejection, Reply};

use crate::caches::{Cache, CachedResponse, CACHES};
use crate::models::{ListParams, MerchandiseList};
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
                let merchandise_list = MerchandiseList::get(&env.db, id).await?;
                let reply = T::from_serializable(&merchandise_list)?;
                let reply = with_status(reply, StatusCode::OK);
                Ok(reply)
            })
            .await?;
        Ok(Box::new(check_etag(etag, response)))
    }

    match accept {
        Some(accept) if accept.accepts_bincode() => {
            get::<ETagReply<Bincode>>(id, etag, env, &CACHES.merchandise_list_bin).await
        }
        _ => get::<ETagReply<Json>>(id, etag, env, &CACHES.merchandise_list).await,
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
                let merchandise_list = MerchandiseList::get_by_shop_id(&env.db, shop_id).await?;
                let reply = T::from_serializable(&merchandise_list)?;
                let reply = with_status(reply, StatusCode::OK);
                Ok(reply)
            })
            .await?;
        Ok(Box::new(check_etag(etag, response)))
    }

    match accept {
        Some(accept) if accept.accepts_bincode() => {
            get::<ETagReply<Bincode>>(shop_id, etag, env, &CACHES.merchandise_list_by_shop_id_bin)
                .await
        }
        _ => get::<ETagReply<Json>>(shop_id, etag, env, &CACHES.merchandise_list_by_shop_id).await,
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
                let merchandise_lists = MerchandiseList::list(&env.db, &list_params).await?;
                let reply = T::from_serializable(&merchandise_lists)?;
                let reply = with_status(reply, StatusCode::OK);
                Ok(reply)
            })
            .await?;
        Ok(Box::new(check_etag(etag, response)))
    }

    match accept {
        Some(accept) if accept.accepts_bincode() => {
            get::<ETagReply<Bincode>>(list_params, etag, env, &CACHES.list_merchandise_lists_bin)
                .await
        }
        _ => get::<ETagReply<Json>>(list_params, etag, env, &CACHES.list_merchandise_lists).await,
    }
}

pub async fn create(
    merchandise_list: MerchandiseList,
    api_key: Option<Uuid>,
    content_type: Option<Mime>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    async fn create<'a, T: DataReply + 'a>(
        merchandise_list: MerchandiseList,
        api_key: Option<Uuid>,
        env: Environment,
    ) -> Result<Box<dyn Reply + 'a>, Rejection> {
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
        let reply = T::from_serializable(&saved_merchandise_list).map_err(reject_anyhow)?;
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
        Ok(Box::new(reply))
    }

    match content_type {
        Some(content_type) if content_type == mime::APPLICATION_OCTET_STREAM => {
            create::<ETagReply<Bincode>>(merchandise_list, api_key, env).await
        }
        _ => create::<ETagReply<Json>>(merchandise_list, api_key, env).await,
    }
}

pub async fn update(
    id: i32,
    merchandise_list: MerchandiseList,
    api_key: Option<Uuid>,
    content_type: Option<Mime>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    async fn update<'a, T: DataReply + 'a>(
        id: i32,
        merchandise_list: MerchandiseList,
        api_key: Option<Uuid>,
        env: Environment,
    ) -> Result<Box<dyn Reply + 'a>, Rejection> {
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
        let reply = T::from_serializable(&updated_merchandise_list).map_err(reject_anyhow)?;
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
        Ok(Box::new(reply))
    }

    match content_type {
        Some(content_type) if content_type == mime::APPLICATION_OCTET_STREAM => {
            update::<ETagReply<Bincode>>(id, merchandise_list, api_key, env).await
        }
        _ => update::<ETagReply<Json>>(id, merchandise_list, api_key, env).await,
    }
}

pub async fn update_by_shop_id(
    shop_id: i32,
    merchandise_list: MerchandiseList,
    api_key: Option<Uuid>,
    content_type: Option<Mime>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    async fn update<'a, T: DataReply + 'a>(
        shop_id: i32,
        merchandise_list: MerchandiseList,
        api_key: Option<Uuid>,
        env: Environment,
    ) -> Result<Box<dyn Reply + 'a>, Rejection> {
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
        let reply = ETagReply::<Json>::from_serializable(&updated_merchandise_list)
            .map_err(reject_anyhow)?;
        let reply = with_header(reply, "Location", url.as_str());
        let reply = with_status(reply, StatusCode::CREATED);
        tokio::spawn(async move {
            let id = updated_merchandise_list
                .id
                .expect("saved merchandise_list has no id");
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
        Ok(Box::new(reply))
    }

    match content_type {
        Some(content_type) if content_type == mime::APPLICATION_OCTET_STREAM => {
            update::<ETagReply<Bincode>>(shop_id, merchandise_list, api_key, env).await
        }
        _ => update::<ETagReply<Json>>(shop_id, merchandise_list, api_key, env).await,
    }
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
