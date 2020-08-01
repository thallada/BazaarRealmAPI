use anyhow::{anyhow, Result};
use ipnetwork::IpNetwork;
use sqlx::postgres::PgPool;
use std::net::SocketAddr;
use tracing::instrument;
use uuid::Uuid;
use warp::http::StatusCode;
use warp::reply::{json, with_header, with_status};
use warp::{Rejection, Reply};

use super::models::{InteriorRefList, ListParams, Model, Owner, Shop};
use super::problem::{forbidden_no_api_key, forbidden_no_owner, reject_anyhow};
use super::Environment;
use crate::caches::Cache;

#[instrument(level = "debug", skip(db, cache, api_key))]
pub async fn authenticate(
    db: &PgPool,
    cache: &Cache<Uuid, i32>,
    api_key: Option<Uuid>,
) -> Result<i32> {
    if let Some(api_key) = api_key {
        cache
            .get(api_key, || async {
                Ok(
                    sqlx::query!("SELECT id FROM owners WHERE api_key = $1", api_key)
                        .fetch_one(db)
                        .await
                        .map_err(|error| {
                            if let sqlx::Error::RowNotFound = error {
                                return forbidden_no_owner();
                            }
                            anyhow!(error)
                        })?
                        .id,
                )
            })
            .await
    } else {
        // TODO: this should be 401 status instead
        Err(forbidden_no_api_key())
    }
}

pub async fn get_shop(id: i32, env: Environment) -> Result<impl Reply, Rejection> {
    env.caches
        .shop
        .get_response(id, || async {
            let shop = Shop::get(&env.db, id).await?;
            let reply = json(&shop);
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await
}

pub async fn list_shops(
    list_params: ListParams,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    env.caches
        .list_shops
        .get_response(list_params.clone(), || async {
            let shops = Shop::list(&env.db, &list_params).await?;
            let reply = json(&shops);
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await
}

pub async fn create_shop(shop: Shop, env: Environment) -> Result<impl Reply, Rejection> {
    // TODO: authenticate
    // TODO: return 400 error with message if unique key is violated
    let saved_shop = shop.save(&env.db).await.map_err(reject_anyhow)?;
    let url = saved_shop.url(&env.api_url).map_err(reject_anyhow)?;
    let reply = json(&saved_shop);
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    env.caches.list_shops.clear().await;
    Ok(reply)
}

pub async fn delete_shop(
    id: i32,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(&env.db, &env.caches.owner_ids_by_api_key, api_key)
        .await
        .map_err(reject_anyhow)?;
    Shop::delete(&env.db, owner_id, id)
        .await
        .map_err(reject_anyhow)?;
    env.caches
        .shop
        .delete_response(id)
        .await
        .map_err(reject_anyhow)?;
    env.caches.list_shops.clear().await;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_owner(id: i32, env: Environment) -> Result<impl Reply, Rejection> {
    env.caches
        .owner
        .get_response(id, || async {
            let owner = Owner::get(&env.db, id).await?;
            let reply = json(&owner);
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await
}

pub async fn list_owners(
    list_params: ListParams,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    env.caches
        .list_owners
        .get_response(list_params.clone(), || async {
            let owners = Owner::list(&env.db, &list_params).await?;
            let reply = json(&owners);
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await
}

pub async fn create_owner(
    owner: Owner,
    remote_addr: Option<SocketAddr>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    // TODO: authenticate and get api_key from header
    let owner_with_ip = match remote_addr {
        Some(addr) => Owner {
            ip_address: Some(IpNetwork::from(addr.ip())),
            ..owner
        },
        None => owner,
    };
    let saved_owner = owner_with_ip.save(&env.db).await.map_err(reject_anyhow)?;
    let url = saved_owner.url(&env.api_url).map_err(reject_anyhow)?;
    let reply = json(&saved_owner);
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    env.caches.list_owners.clear().await;
    Ok(reply)
}

pub async fn delete_owner(
    id: i32,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(&env.db, &env.caches.owner_ids_by_api_key, api_key)
        .await
        .map_err(reject_anyhow)?;
    Owner::delete(&env.db, owner_id, id)
        .await
        .map_err(reject_anyhow)?;
    env.caches
        .owner
        .delete_response(id)
        .await
        .map_err(reject_anyhow)?;
    env.caches
        .owner_ids_by_api_key
        .delete(api_key.expect("api-key has been validated during authenticate"))
        .await
        .map_err(reject_anyhow)?;
    env.caches.list_owners.clear().await;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_interior_ref_list(id: i32, env: Environment) -> Result<impl Reply, Rejection> {
    env.caches
        .interior_ref_list
        .get_response(id, || async {
            let interior_ref_list = InteriorRefList::get(&env.db, id).await?;
            let reply = json(&interior_ref_list);
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await
}

pub async fn list_interior_ref_lists(
    list_params: ListParams,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    env.caches
        .list_interior_ref_lists
        .get_response(list_params.clone(), || async {
            let interior_ref_lists = InteriorRefList::list(&env.db, &list_params).await?;
            let reply = json(&interior_ref_lists);
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await
}

pub async fn create_interior_ref_list(
    interior_ref_list: InteriorRefList,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    // TODO: authenticate
    let saved_interior_ref_list = interior_ref_list
        .save(&env.db)
        .await
        .map_err(reject_anyhow)?;
    let url = saved_interior_ref_list
        .url(&env.api_url)
        .map_err(reject_anyhow)?;
    let reply = json(&saved_interior_ref_list);
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    env.caches.list_interior_ref_lists.clear().await;
    Ok(reply)
}

pub async fn delete_interior_ref_list(
    id: i32,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(&env.db, &env.caches.owner_ids_by_api_key, api_key)
        .await
        .map_err(reject_anyhow)?;
    InteriorRefList::delete(&env.db, owner_id, id)
        .await
        .map_err(reject_anyhow)?;
    env.caches
        .interior_ref_list
        .delete_response(id)
        .await
        .map_err(reject_anyhow)?;
    env.caches.list_interior_ref_lists.clear().await;
    Ok(StatusCode::NO_CONTENT)
}
