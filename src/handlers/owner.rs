use anyhow::Result;
use http::StatusCode;
use ipnetwork::IpNetwork;
use mime::Mime;
use std::net::SocketAddr;
use uuid::Uuid;
use warp::reply::{with_header, with_status};
use warp::{Rejection, Reply};

use crate::caches::CACHES;
use crate::models::{ListParams, Owner, PostedOwner, UnsavedOwner};
use crate::problem::{reject_anyhow, unauthorized_no_api_key};
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
        Some(accept) if accept.accepts_bincode() => (ContentType::Bincode, &CACHES.owner_bin),
        _ => (ContentType::Json, &CACHES.owner),
    };
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
    let (content_type, cache) = match accept {
        Some(accept) if accept.accepts_bincode() => (ContentType::Bincode, &CACHES.list_owners_bin),
        _ => (ContentType::Json, &CACHES.list_owners),
    };
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
    owner: PostedOwner,
    remote_addr: Option<SocketAddr>,
    api_key: Option<Uuid>,
    real_ip: Option<IpNetwork>,
    content_type: Option<Mime>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    if let Some(api_key) = api_key {
        let content_type = match content_type {
            Some(content_type) if content_type == mime::APPLICATION_OCTET_STREAM => {
                ContentType::Bincode
            }
            _ => ContentType::Json,
        };
        let unsaved_owner = UnsavedOwner {
            api_key,
            ip_address: match remote_addr {
                Some(addr) => Some(IpNetwork::from(addr.ip())),
                None => real_ip,
            },
            name: owner.name,
            mod_version: owner.mod_version,
        };
        let saved_owner = Owner::create(unsaved_owner, &env.db)
            .await
            .map_err(reject_anyhow)?;
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
    owner: PostedOwner,
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
