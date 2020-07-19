use serde::de::DeserializeOwned;
use std::convert::Infallible;
use warp::{Filter, Rejection, Reply};

use super::handlers;
use super::models::{InteriorRef, ListParams, Owner, Shop};
use super::Environment;

pub fn get_shop(env: Environment) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("shops" / i32)
        .and(warp::get())
        .and(with_env(env))
        .and_then(handlers::get_shop)
}

pub fn create_shop(
    env: Environment,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("shops")
        .and(warp::post())
        .and(json_body::<Shop>())
        .and(with_env(env))
        .and_then(handlers::create_shop)
}

pub fn list_shops(
    env: Environment,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("shops")
        .and(warp::get())
        .and(warp::query::<ListParams>())
        .and(with_env(env))
        .and_then(handlers::list_shops)
}

pub fn get_owner(env: Environment) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("owners" / i32)
        .and(warp::get())
        .and(with_env(env))
        .and_then(handlers::get_owner)
}

pub fn create_owner(
    env: Environment,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("owners")
        .and(warp::post())
        .and(json_body::<Owner>())
        .and(warp::addr::remote())
        .and(with_env(env))
        .and_then(handlers::create_owner)
}

pub fn list_owners(
    env: Environment,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("owners")
        .and(warp::get())
        .and(warp::query::<ListParams>())
        .and(with_env(env))
        .and_then(handlers::list_owners)
}

pub fn get_interior_ref(
    env: Environment,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("interior_refs" / i32)
        .and(warp::get())
        .and(with_env(env))
        .and_then(handlers::get_interior_ref)
}

pub fn create_interior_ref(
    env: Environment,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("interior_refs")
        .and(warp::post())
        .and(json_body::<InteriorRef>())
        .and(with_env(env))
        .and_then(handlers::create_interior_ref)
}

pub fn list_interior_refs(
    env: Environment,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("interior_refs")
        .and(warp::get())
        .and(warp::query::<ListParams>())
        .and(with_env(env))
        .and_then(handlers::list_interior_refs)
}

pub fn bulk_create_interior_refs(
    env: Environment,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("interior_refs" / "bulk")
        .and(warp::post())
        .and(json_body::<Vec<InteriorRef>>())
        .and(with_env(env))
        .and_then(handlers::bulk_create_interior_refs)
}

fn with_env(env: Environment) -> impl Filter<Extract = (Environment,), Error = Infallible> + Clone {
    warp::any().map(move || env.clone())
}

fn json_body<T>() -> impl Filter<Extract = (T,), Error = warp::Rejection> + Clone
where
    T: Send + DeserializeOwned,
{
    warp::body::content_length_limit(1024 * 64).and(warp::body::json())
}
