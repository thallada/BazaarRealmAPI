use serde::de::DeserializeOwned;
use std::convert::Infallible;
use warp::{Filter, Rejection, Reply};

use super::handlers;
use super::models::{InteriorRefList, ListParams, Owner, Shop};
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

pub fn get_interior_ref_list(
    env: Environment,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("interior_ref_lists" / i32)
        .and(warp::get())
        .and(with_env(env))
        .and_then(handlers::get_interior_ref_list)
}

pub fn create_interior_ref_list(
    env: Environment,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("interior_ref_lists")
        .and(warp::post())
        .and(json_body::<InteriorRefList>())
        .and(with_env(env))
        .and_then(handlers::create_interior_ref_list)
}

pub fn list_interior_ref_lists(
    env: Environment,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("interior_ref_lists")
        .and(warp::get())
        .and(warp::query::<ListParams>())
        .and(with_env(env))
        .and_then(handlers::list_interior_ref_lists)
}

pub fn bulk_create_interior_ref_lists(
    env: Environment,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("interior_ref_lists" / "bulk")
        .and(warp::post())
        .and(json_body::<Vec<InteriorRefList>>())
        .and(with_env(env))
        .and_then(handlers::bulk_create_interior_ref_lists)
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
