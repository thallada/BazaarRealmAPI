use serde::de::DeserializeOwned;
use std::convert::Infallible;
use warp::{Filter, Rejection, Reply};

use super::handlers;
use super::models::{InteriorRefList, ListParams, Owner, Shop};
use super::Environment;

pub fn shops(env: Environment) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("shops").and(
        get_shop(env.clone())
            .or(delete_shop(env.clone()))
            .or(create_shop(env.clone()))
            .or(list_shops(env)),
    )
}

pub fn owners(env: Environment) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("owners").and(
        get_owner(env.clone())
            .or(delete_owner(env.clone()))
            .or(create_owner(env.clone()))
            .or(list_owners(env)),
    )
}

pub fn interior_ref_lists(
    env: Environment,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("interior_ref_lists").and(
        get_interior_ref_list(env.clone())
            .or(delete_interior_ref_list(env.clone()))
            .or(create_interior_ref_list(env.clone()))
            .or(list_interior_ref_lists(env)),
    )
}

pub fn get_shop(env: Environment) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path::param()
        .and(warp::get())
        .and(with_env(env))
        .and_then(handlers::get_shop)
}

pub fn create_shop(
    env: Environment,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::post()
        .and(json_body::<Shop>())
        .and(with_env(env))
        .and_then(handlers::create_shop)
}

pub fn delete_shop(
    env: Environment,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path::param()
        .and(warp::delete())
        .and(warp::header::optional("api-key"))
        .and(with_env(env))
        .and_then(handlers::delete_shop)
}

pub fn list_shops(
    env: Environment,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::get()
        .and(warp::query::<ListParams>())
        .and(with_env(env))
        .and_then(handlers::list_shops)
}

pub fn get_owner(env: Environment) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path::param()
        .and(warp::get())
        .and(with_env(env))
        .and_then(handlers::get_owner)
}

pub fn create_owner(
    env: Environment,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::post()
        .and(json_body::<Owner>())
        .and(warp::addr::remote())
        .and(with_env(env))
        .and_then(handlers::create_owner)
}

pub fn delete_owner(
    env: Environment,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path::param()
        .and(warp::delete())
        .and(warp::header::optional("api-key"))
        .and(with_env(env))
        .and_then(handlers::delete_owner)
}

pub fn list_owners(
    env: Environment,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::get()
        .and(warp::query::<ListParams>())
        .and(with_env(env))
        .and_then(handlers::list_owners)
}

pub fn get_interior_ref_list(
    env: Environment,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path::param()
        .and(warp::get())
        .and(with_env(env))
        .and_then(handlers::get_interior_ref_list)
}

pub fn create_interior_ref_list(
    env: Environment,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::post()
        .and(json_body::<InteriorRefList>())
        .and(with_env(env))
        .and_then(handlers::create_interior_ref_list)
}

pub fn delete_interior_ref_list(
    env: Environment,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path::param()
        .and(warp::delete())
        .and(warp::header::optional("api-key"))
        .and(with_env(env))
        .and_then(handlers::delete_interior_ref_list)
}

pub fn list_interior_ref_lists(
    env: Environment,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::get()
        .and(warp::query::<ListParams>())
        .and(with_env(env))
        .and_then(handlers::list_interior_ref_lists)
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
