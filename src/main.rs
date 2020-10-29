use anyhow::Result;
use dotenv::dotenv;
use http::StatusCode;
use hyper::server::Server;
use listenfd::ListenFd;
use serde::{de::DeserializeOwned, Serialize};
use sqlx::postgres::PgPool;
use std::convert::Infallible;
use std::env;
use tracing::info;
use tracing_subscriber::fmt::format::FmtSpan;
use url::Url;
use warp::Filter;

mod caches;
mod handlers;
#[macro_use]
mod macros;
mod models;
mod problem;

use caches::Caches;
use models::interior_ref_list::InteriorRefList;
use models::merchandise_list::{MerchandiseList, MerchandiseParams};
use models::owner::Owner;
use models::shop::Shop;
use models::ListParams;

#[derive(Debug, Clone)]
pub struct Environment {
    pub db: PgPool,
    pub caches: Caches,
    pub api_url: Url,
}

impl Environment {
    async fn new(api_url: Url) -> Result<Environment> {
        Ok(Environment {
            db: PgPool::builder()
                .max_size(5)
                .build(&env::var("DATABASE_URL")?)
                .await?,
            caches: Caches::initialize(),
            api_url,
        })
    }
}

#[derive(Serialize)]
struct ErrorMessage {
    code: u16,
    message: String,
}

fn with_env(env: Environment) -> impl Filter<Extract = (Environment,), Error = Infallible> + Clone {
    warp::any().map(move || env.clone())
}

fn json_body<T>() -> impl Filter<Extract = (T,), Error = warp::Rejection> + Clone
where
    T: Send + DeserializeOwned,
{
    warp::body::content_length_limit(1024 * 1024).and(warp::body::json())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let env_log_filter =
        env::var("RUST_LOG").unwrap_or_else(|_| "warp=info,bazaar_realm_api=info".to_owned());
    tracing_subscriber::fmt()
        .with_env_filter(env_log_filter)
        .with_span_events(FmtSpan::CLOSE)
        .init();

    let host = env::var("HOST").expect("`HOST` environment variable not defined");
    let host_url = Url::parse(&host).expect("Cannot parse URL from `HOST` environment variable");
    let api_url = host_url.join("/v1/")?;
    let env = Environment::new(api_url).await?;

    let status_handler = warp::path::path("status")
        .and(warp::path::end())
        .and(warp::get())
        .map(|| StatusCode::OK); // TODO: return what api versions this server supports instead
    let get_owner_handler = warp::path("owners").and(
        warp::path::param()
            .and(warp::path::end())
            .and(warp::get())
            .and(with_env(env.clone()))
            .and_then(handlers::get_owner),
    );
    let create_owner_handler = warp::path("owners").and(
        warp::path::end()
            .and(warp::post())
            .and(json_body::<Owner>())
            .and(warp::addr::remote())
            .and(warp::header::optional("api-key"))
            .and(warp::header::optional("x-real-ip"))
            .and(with_env(env.clone()))
            .and_then(handlers::create_owner),
    );
    let delete_owner_handler = warp::path("owners").and(
        warp::path::param()
            .and(warp::path::end())
            .and(warp::delete())
            .and(warp::header::optional("api-key"))
            .and(with_env(env.clone()))
            .and_then(handlers::delete_owner),
    );
    let update_owner_handler = warp::path("owners").and(
        warp::path::param()
            .and(warp::path::end())
            .and(warp::patch())
            .and(json_body::<Owner>())
            .and(warp::header::optional("api-key"))
            .and(with_env(env.clone()))
            .and_then(handlers::update_owner),
    );
    let list_owners_handler = warp::path("owners").and(
        warp::path::end()
            .and(warp::get())
            .and(warp::query::<ListParams>())
            .and(with_env(env.clone()))
            .and_then(handlers::list_owners),
    );
    let get_shop_handler = warp::path("shops").and(
        warp::path::param()
            .and(warp::path::end())
            .and(warp::get())
            .and(with_env(env.clone()))
            .and_then(handlers::get_shop),
    );
    let create_shop_handler = warp::path("shops").and(
        warp::path::end()
            .and(warp::post())
            .and(json_body::<Shop>())
            .and(warp::header::optional("api-key"))
            .and(with_env(env.clone()))
            .and_then(handlers::create_shop),
    );
    let delete_shop_handler = warp::path("shops").and(
        warp::path::param()
            .and(warp::path::end())
            .and(warp::delete())
            .and(warp::header::optional("api-key"))
            .and(with_env(env.clone()))
            .and_then(handlers::delete_shop),
    );
    let update_shop_handler = warp::path("shops").and(
        warp::path::param()
            .and(warp::path::end())
            .and(warp::patch())
            .and(json_body::<Shop>())
            .and(warp::header::optional("api-key"))
            .and(with_env(env.clone()))
            .and_then(handlers::update_shop),
    );
    let list_shops_handler = warp::path("shops").and(
        warp::path::end()
            .and(warp::get())
            .and(warp::query::<ListParams>())
            .and(with_env(env.clone()))
            .and_then(handlers::list_shops),
    );
    let get_interior_ref_list_handler = warp::path("interior_ref_lists").and(
        warp::path::param()
            .and(warp::path::end())
            .and(warp::get())
            .and(with_env(env.clone()))
            .and_then(handlers::get_interior_ref_list),
    );
    let create_interior_ref_list_handler = warp::path("interior_ref_lists").and(
        warp::path::end()
            .and(warp::post())
            .and(json_body::<InteriorRefList>())
            .and(warp::header::optional("api-key"))
            .and(with_env(env.clone()))
            .and_then(handlers::create_interior_ref_list),
    );
    let delete_interior_ref_list_handler = warp::path("interior_ref_lists").and(
        warp::path::param()
            .and(warp::path::end())
            .and(warp::delete())
            .and(warp::header::optional("api-key"))
            .and(with_env(env.clone()))
            .and_then(handlers::delete_interior_ref_list),
    );
    let update_interior_ref_list_handler = warp::path("interior_ref_lists").and(
        warp::path::param()
            .and(warp::path::end())
            .and(warp::patch())
            .and(json_body::<InteriorRefList>())
            .and(warp::header::optional("api-key"))
            .and(with_env(env.clone()))
            .and_then(handlers::update_interior_ref_list),
    );
    let update_interior_ref_list_by_shop_id_handler = warp::path("shops").and(
        warp::path::param()
            .and(warp::path("interior_ref_list"))
            .and(warp::path::end())
            .and(warp::patch())
            .and(json_body::<InteriorRefList>())
            .and(warp::header::optional("api-key"))
            .and(with_env(env.clone()))
            .and_then(handlers::update_interior_ref_list_by_shop_id),
    );
    let list_interior_ref_lists_handler = warp::path("interior_ref_lists").and(
        warp::path::end()
            .and(warp::get())
            .and(warp::query::<ListParams>())
            .and(with_env(env.clone()))
            .and_then(handlers::list_interior_ref_lists),
    );
    let get_interior_ref_list_by_shop_id_handler = warp::path("shops").and(
        warp::path::param()
            .and(warp::path("interior_ref_list"))
            .and(warp::path::end())
            .and(warp::get())
            .and(with_env(env.clone()))
            .and_then(handlers::get_interior_ref_list_by_shop_id),
    );
    let get_merchandise_list_handler = warp::path("merchandise_lists").and(
        warp::path::param()
            .and(warp::path::end())
            .and(warp::get())
            .and(with_env(env.clone()))
            .and_then(handlers::get_merchandise_list),
    );
    let create_merchandise_list_handler = warp::path("merchandise_lists").and(
        warp::path::end()
            .and(warp::post())
            .and(json_body::<MerchandiseList>())
            .and(warp::header::optional("api-key"))
            .and(with_env(env.clone()))
            .and_then(handlers::create_merchandise_list),
    );
    let delete_merchandise_list_handler = warp::path("merchandise_lists").and(
        warp::path::param()
            .and(warp::path::end())
            .and(warp::delete())
            .and(warp::header::optional("api-key"))
            .and(with_env(env.clone()))
            .and_then(handlers::delete_merchandise_list),
    );
    let update_merchandise_list_handler = warp::path("merchandise_lists").and(
        warp::path::param()
            .and(warp::path::end())
            .and(warp::patch())
            .and(json_body::<MerchandiseList>())
            .and(warp::header::optional("api-key"))
            .and(with_env(env.clone()))
            .and_then(handlers::update_merchandise_list),
    );
    let update_merchandise_list_by_shop_id_handler = warp::path("shops").and(
        warp::path::param()
            .and(warp::path("merchandise_list"))
            .and(warp::path::end())
            .and(warp::patch())
            .and(json_body::<MerchandiseList>())
            .and(warp::header::optional("api-key"))
            .and(with_env(env.clone()))
            .and_then(handlers::update_merchandise_list_by_shop_id),
    );
    let list_merchandise_lists_handler = warp::path("merchandise_lists").and(
        warp::path::end()
            .and(warp::get())
            .and(warp::query::<ListParams>())
            .and(with_env(env.clone()))
            .and_then(handlers::list_merchandise_lists),
    );
    let get_merchandise_list_by_shop_id_handler = warp::path("shops").and(
        warp::path::param()
            .and(warp::path("merchandise_list"))
            .and(warp::path::end())
            .and(warp::get())
            .and(with_env(env.clone()))
            .and_then(handlers::get_merchandise_list_by_shop_id),
    );
    let buy_merchandise_handler = warp::path("shops").and(
        warp::path::param()
            .and(warp::path("merchandise_list"))
            .and(warp::path::end())
            .and(warp::post())
            .and(warp::query::<MerchandiseParams>())
            .and(warp::header::optional("api-key"))
            .and(with_env(env.clone()))
            .and_then(handlers::buy_merchandise),
    );

    let routes = warp::path("v1")
        .and(balanced_or_tree!(
            status_handler,
            get_owner_handler,
            delete_owner_handler,
            update_owner_handler,
            create_owner_handler,
            list_owners_handler,
            get_shop_handler,
            delete_shop_handler,
            update_shop_handler,
            create_shop_handler,
            list_shops_handler,
            get_interior_ref_list_by_shop_id_handler,
            get_merchandise_list_by_shop_id_handler,
            update_interior_ref_list_by_shop_id_handler,
            update_merchandise_list_by_shop_id_handler,
            buy_merchandise_handler,
            get_interior_ref_list_handler,
            delete_interior_ref_list_handler,
            update_interior_ref_list_handler,
            create_interior_ref_list_handler,
            list_interior_ref_lists_handler,
            get_merchandise_list_handler,
            delete_merchandise_list_handler,
            update_merchandise_list_handler,
            create_merchandise_list_handler,
            list_merchandise_lists_handler,
            // warp::any().map(|| StatusCode::NOT_FOUND),
        ))
        .recover(problem::unpack_problem)
        .with(warp::compression::gzip())
        .with(warp::trace::request());

    let svc = warp::service(routes);
    let make_svc = hyper::service::make_service_fn(|_: _| {
        let svc = svc.clone();
        async move { Ok::<_, Infallible>(svc) }
    });

    let mut listenfd = ListenFd::from_env();
    let server = if let Some(l) = listenfd.take_tcp_listener(0)? {
        Server::from_tcp(l)?
    } else {
        Server::bind(&([127, 0, 0, 1], 3030).into())
    };

    // warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
    server.serve(make_svc).await?;
    Ok(())
}
