use ipnetwork::IpNetwork;
use std::net::SocketAddr;
use warp::http::StatusCode;
use warp::reply::{json, with_header, with_status};
use warp::{Rejection, Reply};

use super::models::{InteriorRef, ListParams, Model, Owner, Shop};
use super::problem::reject_anyhow;
use super::Environment;

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

pub async fn get_interior_ref(id: i32, env: Environment) -> Result<impl Reply, Rejection> {
    let interior_ref = InteriorRef::get(&env.db, id).await.map_err(reject_anyhow)?;
    let reply = json(&interior_ref);
    let reply = with_status(reply, StatusCode::OK);
    Ok(reply)
}

pub async fn list_interior_refs(
    list_params: ListParams,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let interior_refs = InteriorRef::list(&env.db, list_params)
        .await
        .map_err(reject_anyhow)?;
    let reply = json(&interior_refs);
    let reply = with_status(reply, StatusCode::OK);
    Ok(reply)
}

pub async fn create_interior_ref(
    interior_ref: InteriorRef,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let saved_interior_ref = interior_ref.save(&env.db).await.map_err(reject_anyhow)?;
    let url = saved_interior_ref
        .url(&env.api_url)
        .map_err(reject_anyhow)?;
    let reply = json(&saved_interior_ref);
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    Ok(reply)
}

pub async fn bulk_create_interior_refs(
    interior_refs: Vec<InteriorRef>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    InteriorRef::bulk_save(&env.db, interior_refs)
        .await
        .map_err(reject_anyhow)?;
    Ok(StatusCode::CREATED)
}
