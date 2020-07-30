use anyhow::{anyhow, Result};
use ipnetwork::IpNetwork;
use sqlx::postgres::PgPool;
use std::net::SocketAddr;
use uuid::Uuid;
use warp::http::StatusCode;
use warp::reply::{json, with_header, with_status};
use warp::{Rejection, Reply};

use super::models::{InteriorRefList, ListParams, Model, Owner, Shop};
use super::problem::{forbidden_no_api_key, forbidden_no_owner, reject_anyhow};
use super::Environment;

pub async fn authenticate(api_key: Option<Uuid>, db: &PgPool) -> Result<i32> {
    if let Some(api_key) = api_key {
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
    } else {
        Err(forbidden_no_api_key())
    }
}

pub async fn get_shop(id: i32, env: Environment) -> Result<impl Reply, Rejection> {
    let shop = Shop::get(&env.db, id).await.map_err(reject_anyhow)?;
    let reply = json(&shop);
    let reply = with_status(reply, StatusCode::OK);
    Ok(reply)
}

pub async fn list_shops(
    list_params: ListParams,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let shops = Shop::list(&env.db, list_params)
        .await
        .map_err(reject_anyhow)?;
    let reply = json(&shops);
    let reply = with_status(reply, StatusCode::OK);
    Ok(reply)
}

pub async fn create_shop(shop: Shop, env: Environment) -> Result<impl Reply, Rejection> {
    let saved_shop = shop.save(&env.db).await.map_err(reject_anyhow)?;
    let url = saved_shop.url(&env.api_url).map_err(reject_anyhow)?;
    let reply = json(&saved_shop);
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    Ok(reply)
}

pub async fn delete_shop(
    id: i32,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(api_key, &env.db)
        .await
        .map_err(reject_anyhow)?;
    dbg!(owner_id);
    Shop::delete(&env.db, owner_id, id)
        .await
        .map_err(reject_anyhow)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_owner(id: i32, env: Environment) -> Result<impl Reply, Rejection> {
    let owner = Owner::get(&env.db, id).await.map_err(reject_anyhow)?;
    let reply = json(&owner);
    let reply = with_status(reply, StatusCode::OK);
    Ok(reply)
}

pub async fn list_owners(
    list_params: ListParams,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owners = Owner::list(&env.db, list_params)
        .await
        .map_err(reject_anyhow)?;
    let reply = json(&owners);
    let reply = with_status(reply, StatusCode::OK);
    Ok(reply)
}

pub async fn create_owner(
    owner: Owner,
    remote_addr: Option<SocketAddr>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
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
    Ok(reply)
}

pub async fn delete_owner(
    id: i32,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(api_key, &env.db)
        .await
        .map_err(reject_anyhow)?;
    dbg!(owner_id);
    Owner::delete(&env.db, owner_id, id)
        .await
        .map_err(reject_anyhow)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_interior_ref_list(id: i32, env: Environment) -> Result<impl Reply, Rejection> {
    let interior_ref_list = InteriorRefList::get(&env.db, id)
        .await
        .map_err(reject_anyhow)?;
    let reply = json(&interior_ref_list);
    let reply = with_status(reply, StatusCode::OK);
    Ok(reply)
}

pub async fn list_interior_ref_lists(
    list_params: ListParams,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let interior_ref_lists = InteriorRefList::list(&env.db, list_params)
        .await
        .map_err(reject_anyhow)?;
    let reply = json(&interior_ref_lists);
    let reply = with_status(reply, StatusCode::OK);
    Ok(reply)
}

pub async fn create_interior_ref_list(
    interior_ref_list: InteriorRefList,
    env: Environment,
) -> Result<impl Reply, Rejection> {
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
    Ok(reply)
}

pub async fn delete_interior_ref_list(
    id: i32,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(api_key, &env.db)
        .await
        .map_err(reject_anyhow)?;
    dbg!(owner_id);
    InteriorRefList::delete(&env.db, owner_id, id)
        .await
        .map_err(reject_anyhow)?;
    Ok(StatusCode::NO_CONTENT)
}
