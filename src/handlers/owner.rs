use anyhow::Result;
use http::StatusCode;
use hyper::body::Bytes;
use ipnetwork::IpNetwork;
use mime::Mime;
use std::net::SocketAddr;
use uuid::Uuid;
use warp::reply::{with_header, with_status};
use warp::{Rejection, Reply};

use crate::caches::{CachedResponse, CACHES};
use crate::models::{FullPostedOwner, ListParams, Owner, PostedOwner};
use crate::problem::{reject_anyhow, unauthorized_no_api_key};
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
    } = TypedCache::<i32, CachedResponse>::pick_cache(accept, &CACHES.owner_bin, &CACHES.owner);
    let response = cache
        .get_response(id, || async {
            let owner = Owner::get(&env.db, id).await?;
            let reply: Box<dyn Reply> = match content_type {
                ContentType::Bincode => Box::new(ETagReply::<Bincode>::from_serializable(&owner)?),
                ContentType::Json => Box::new(ETagReply::<Json>::from_serializable(&owner)?),
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
        &CACHES.list_owners_bin,
        &CACHES.list_owners,
    );
    let response = cache
        .get_response(list_params.clone(), || async {
            let owners = Owner::list(&env.db, &list_params).await?;
            let reply: Box<dyn Reply> = match content_type {
                ContentType::Bincode => Box::new(ETagReply::<Bincode>::from_serializable(&owners)?),
                ContentType::Json => Box::new(ETagReply::<Json>::from_serializable(&owners)?),
            };
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await?;
    Ok(check_etag(etag, response))
}

pub async fn create(
    bytes: Bytes,
    remote_addr: Option<SocketAddr>,
    api_key: Option<Uuid>,
    real_ip: Option<IpNetwork>,
    content_type: Option<Mime>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    if let Some(api_key) = api_key {
        let DeserializedBody {
            body: owner,
            content_type,
        } = DeserializedBody::<PostedOwner>::from_bytes(bytes, content_type)
            .map_err(reject_anyhow)?;
        let owner = FullPostedOwner {
            name: owner.name,
            mod_version: owner.mod_version,
            api_key,
            ip_address: match remote_addr {
                Some(addr) => Some(IpNetwork::from(addr.ip())),
                None => real_ip,
            },
        };
        let saved_owner = Owner::create(owner, &env.db).await.map_err(reject_anyhow)?;
        let url = saved_owner.url(&env.api_url).map_err(reject_anyhow)?;
        let reply: Box<dyn Reply> = match content_type {
            ContentType::Bincode => Box::new(
                ETagReply::<Bincode>::from_serializable(&saved_owner).map_err(reject_anyhow)?,
            ),
            ContentType::Json => {
                Box::new(ETagReply::<Json>::from_serializable(&saved_owner).map_err(reject_anyhow)?)
            }
        };
        let reply = with_header(reply, "Location", url.as_str());
        let reply = with_status(reply, StatusCode::CREATED);
        tokio::spawn(async move {
            CACHES.list_owners.clear().await;
            CACHES.list_owners_bin.clear().await;
        });
        Ok(reply)
    } else {
        Err(reject_anyhow(unauthorized_no_api_key()))
    }
}

pub async fn update(
    id: i32,
    bytes: Bytes,
    api_key: Option<Uuid>,
    content_type: Option<Mime>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let DeserializedBody {
        body: owner,
        content_type,
    } = DeserializedBody::<PostedOwner>::from_bytes(bytes, content_type).map_err(reject_anyhow)?;
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    let updated_owner = Owner::update(owner, &env.db, owner_id, id)
        .await
        .map_err(reject_anyhow)?;
    let url = updated_owner.url(&env.api_url).map_err(reject_anyhow)?;
    let reply: Box<dyn Reply> = match content_type {
        ContentType::Bincode => Box::new(
            ETagReply::<Bincode>::from_serializable(&updated_owner).map_err(reject_anyhow)?,
        ),
        ContentType::Json => {
            Box::new(ETagReply::<Json>::from_serializable(&updated_owner).map_err(reject_anyhow)?)
        }
    };
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    tokio::spawn(async move {
        CACHES.owner.delete_response(id).await;
        CACHES.owner_bin.delete_response(id).await;
        CACHES.list_owners.clear().await;
        CACHES.list_owners_bin.clear().await;
    });
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
