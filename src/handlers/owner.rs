use anyhow::Result;
use http::StatusCode;
use ipnetwork::IpNetwork;
use std::net::SocketAddr;
use uuid::Uuid;
use warp::reply::{with_header, with_status};
use warp::{Rejection, Reply};

use crate::models::{ListParams, Owner};
use crate::problem::{reject_anyhow, unauthorized_no_api_key};
use crate::Environment;

use super::{authenticate, check_etag, JsonWithETag};

pub async fn get(id: i32, etag: Option<String>, env: Environment) -> Result<impl Reply, Rejection> {
    let response = env
        .caches
        .owner
        .get_response(id, || async {
            let owner = Owner::get(&env.db, id).await?;
            let reply = JsonWithETag::from_serializable(&owner)?;
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
        .list_owners
        .get_response(list_params.clone(), || async {
            let owners = Owner::list(&env.db, &list_params).await?;
            let reply = JsonWithETag::from_serializable(&owners)?;
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await?;
    Ok(check_etag(etag, response))
}

pub async fn create(
    owner: Owner,
    remote_addr: Option<SocketAddr>,
    api_key: Option<Uuid>,
    real_ip: Option<IpNetwork>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    if let Some(api_key) = api_key {
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
        let reply = JsonWithETag::from_serializable(&saved_owner).map_err(reject_anyhow)?;
        let reply = with_header(reply, "Location", url.as_str());
        let reply = with_status(reply, StatusCode::CREATED);
        env.caches.list_owners.clear().await;
        Ok(reply)
    } else {
        Err(reject_anyhow(unauthorized_no_api_key()))
    }
}

pub async fn update(
    id: i32,
    owner: Owner,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
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
    let reply = JsonWithETag::from_serializable(&updated_owner).map_err(reject_anyhow)?;
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    env.caches.owner.delete_response(id).await;
    env.caches.list_owners.clear().await;
    Ok(reply)
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
    env.caches.owner.delete_response(id).await;
    env.caches
        .owner_ids_by_api_key
        .delete(api_key.expect("api-key has been validated during authenticate"))
        .await;
    env.caches.list_owners.clear().await;
    Ok(StatusCode::NO_CONTENT)
}
