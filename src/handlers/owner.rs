use anyhow::Result;
use http::StatusCode;
use ipnetwork::IpNetwork;
use mime::Mime;
use std::net::SocketAddr;
use uuid::Uuid;
use warp::reply::{with_header, with_status};
use warp::{Rejection, Reply};

use crate::caches::{Cache, CachedResponse, CACHES};
use crate::models::{ListParams, Owner};
use crate::problem::{reject_anyhow, unauthorized_no_api_key};
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
                let owner = Owner::get(&env.db, id).await?;
                let reply = T::from_serializable(&owner)?;
                let reply = with_status(reply, StatusCode::OK);
                Ok(reply)
            })
            .await?;
        Ok(Box::new(check_etag(etag, response)))
    }

    match accept {
        Some(accept) if accept.accepts_bincode() => {
            get::<ETagReply<Bincode>>(id, etag, env, &CACHES.owner_bin).await
        }
        _ => get::<ETagReply<Json>>(id, etag, env, &CACHES.owner).await,
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
                let owners = Owner::list(&env.db, &list_params).await?;
                let reply = T::from_serializable(&owners)?;
                let reply = with_status(reply, StatusCode::OK);
                Ok(reply)
            })
            .await?;
        Ok(Box::new(check_etag(etag, response)))
    }

    match accept {
        Some(accept) if accept.accepts_bincode() => {
            get::<ETagReply<Bincode>>(list_params, etag, env, &CACHES.list_owners_bin).await
        }
        _ => get::<ETagReply<Json>>(list_params, etag, env, &CACHES.list_owners).await,
    }
}

pub async fn create(
    owner: Owner,
    remote_addr: Option<SocketAddr>,
    api_key: Option<Uuid>,
    real_ip: Option<IpNetwork>,
    content_type: Option<Mime>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    async fn create<'a, T: DataReply + 'a>(
        owner: Owner,
        remote_addr: Option<SocketAddr>,
        api_key: Uuid,
        real_ip: Option<IpNetwork>,
        env: Environment,
    ) -> Result<Box<dyn Reply + 'a>, Rejection> {
        let owner_with_ip_and_key = match remote_addr {
            Some(addr) => Owner {
                api_key: Some(api_key),
                ip_address: Some(IpNetwork::from(addr.ip())),
                ..owner
            },
            None => Owner {
                api_key: Some(api_key),
                ip_address: real_ip,
                ..owner
            },
        };
        let saved_owner = owner_with_ip_and_key
            .create(&env.db)
            .await
            .map_err(reject_anyhow)?;
        let url = saved_owner.url(&env.api_url).map_err(reject_anyhow)?;
        let reply = T::from_serializable(&saved_owner).map_err(reject_anyhow)?;
        let reply = with_header(reply, "Location", url.as_str());
        let reply = with_status(reply, StatusCode::CREATED);
        tokio::spawn(async move {
            CACHES.list_owners.clear().await;
            CACHES.list_owners_bin.clear().await;
        });
        Ok(Box::new(reply))
    }

    if let Some(api_key) = api_key {
        match content_type {
            Some(content_type) if content_type == mime::APPLICATION_OCTET_STREAM => {
                create::<ETagReply<Bincode>>(owner, remote_addr, api_key, real_ip, env).await
            }
            _ => create::<ETagReply<Json>>(owner, remote_addr, api_key, real_ip, env).await,
        }
    } else {
        Err(reject_anyhow(unauthorized_no_api_key()))
    }
}

pub async fn update(
    id: i32,
    owner: Owner,
    api_key: Option<Uuid>,
    content_type: Option<Mime>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    async fn update<'a, T: DataReply + 'a>(
        id: i32,
        owner: Owner,
        api_key: Option<Uuid>,
        env: Environment,
    ) -> Result<Box<dyn Reply + 'a>, Rejection> {
        let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
        let owner_with_id = Owner {
            id: Some(id),
            ..owner
        };
        let updated_owner = owner_with_id
            .update(&env.db, owner_id, id)
            .await
            .map_err(reject_anyhow)?;
        let url = updated_owner.url(&env.api_url).map_err(reject_anyhow)?;
        let reply = T::from_serializable(&updated_owner).map_err(reject_anyhow)?;
        let reply = with_header(reply, "Location", url.as_str());
        let reply = with_status(reply, StatusCode::CREATED);
        tokio::spawn(async move {
            CACHES.owner.delete_response(id).await;
            CACHES.owner_bin.delete_response(id).await;
            CACHES.list_owners.clear().await;
            CACHES.list_owners_bin.clear().await;
        });
        Ok(Box::new(reply))
    }

    match content_type {
        Some(content_type) if content_type == mime::APPLICATION_OCTET_STREAM => {
            update::<ETagReply<Bincode>>(id, owner, api_key, env).await
        }
        _ => update::<ETagReply<Json>>(id, owner, api_key, env).await,
    }
}

pub async fn delete(
    id: i32,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    Owner::delete(&env.db, owner_id, id)
        .await
        .map_err(reject_anyhow)?;
    tokio::spawn(async move {
        let api_key = api_key.expect("api-key has been validated during authenticate");
        CACHES.owner.delete_response(id).await;
        CACHES.owner_bin.delete_response(id).await;
        CACHES.owner_ids_by_api_key.delete(api_key).await;
        CACHES.list_owners.clear().await;
        CACHES.list_owners_bin.clear().await;
    });
    Ok(StatusCode::NO_CONTENT)
}
