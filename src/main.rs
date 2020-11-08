#[macro_use]
extern crate lazy_static;

use anyhow::Result;
use dotenv::dotenv;
use http::StatusCode;
use hyper::server::Server;
use listenfd::ListenFd;
use serde::{de::DeserializeOwned, Serialize};
use sqlx::postgres::PgPool;
use std::convert::Infallible;
use std::env;
use tracing_subscriber::fmt::format::FmtSpan;
use url::Url;
use warp::Filter;

mod caches;
mod handlers;
#[macro_use]
mod macros;
mod models;
mod problem;

use models::{InteriorRefList, ListParams, MerchandiseList, Owner, Shop, Transaction};

#[derive(Debug, Clone)]
pub struct Environment {
    pub db: PgPool,
    pub api_url: Url,
}

impl Environment {
    async fn new(api_url: Url) -> Result<Environment> {
        Ok(Environment {
            db: PgPool::builder()
                .max_size(5)
                .build(&env::var("DATABASE_URL")?)
                .await?,
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
            .and(warp::header::optional("if-none-match"))
            .and(with_env(env.clone()))
            .and_then(handlers::owner::get),
    );
    let create_owner_handler = warp::path("owners").and(
        warp::path::end()
            .and(warp::post())
            .and(json_body::<Owner>())
            .and(warp::addr::remote())
            .and(warp::header::optional("api-key"))
            .and(warp::header::optional("x-real-ip"))
            .and(with_env(env.clone()))
            .and_then(handlers::owner::create),
    );
    let delete_owner_handler = warp::path("owners").and(
        warp::path::param()
            .and(warp::path::end())
            .and(warp::delete())
            .and(warp::header::optional("api-key"))
            .and(with_env(env.clone()))
            .and_then(handlers::owner::delete),
    );
    let update_owner_handler = warp::path("owners").and(
        warp::path::param()
            .and(warp::path::end())
            .and(warp::patch())
            .and(json_body::<Owner>())
            .and(warp::header::optional("api-key"))
            .and(with_env(env.clone()))
            .and_then(handlers::owner::update),
    );
    let list_owners_handler = warp::path("owners").and(
        warp::path::end()
            .and(warp::get())
            .and(warp::query::<ListParams>())
            .and(warp::header::optional("if-none-match"))
            .and(with_env(env.clone()))
            .and_then(handlers::owner::list),
    );
    let get_shop_handler = warp::path("shops").and(
        warp::path::param()
            .and(warp::path::end())
            .and(warp::get())
            .and(warp::header::optional("if-none-match"))
            .and(with_env(env.clone()))
            .and_then(handlers::shop::get),
    );
    let create_shop_handler = warp::path("shops").and(
        warp::path::end()
            .and(warp::post())
            .and(json_body::<Shop>())
            .and(warp::header::optional("api-key"))
            .and(with_env(env.clone()))
            .and_then(handlers::shop::create),
    );
    let delete_shop_handler = warp::path("shops").and(
        warp::path::param()
            .and(warp::path::end())
            .and(warp::delete())
            .and(warp::header::optional("api-key"))
            .and(with_env(env.clone()))
            .and_then(handlers::shop::delete),
    );
    let update_shop_handler = warp::path("shops").and(
        warp::path::param()
            .and(warp::path::end())
            .and(warp::patch())
            .and(json_body::<Shop>())
            .and(warp::header::optional("api-key"))
            .and(with_env(env.clone()))
            .and_then(handlers::shop::update),
    );
    let list_shops_handler = warp::path("shops").and(
        warp::path::end()
            .and(warp::get())
            .and(warp::query::<ListParams>())
            .and(warp::header::optional("if-none-match"))
            .and(with_env(env.clone()))
            .and_then(handlers::shop::list),
    );
    let get_interior_ref_list_handler = warp::path("interior_ref_lists").and(
        warp::path::param()
            .and(warp::path::end())
            .and(warp::get())
            .and(warp::header::optional("if-none-match"))
            .and(warp::header::optional("accept"))
            .and(with_env(env.clone()))
            .and_then(handlers::interior_ref_list::get),
    );
    let create_interior_ref_list_handler = warp::path("interior_ref_lists").and(
        warp::path::end()
            .and(warp::post())
            .and(json_body::<InteriorRefList>())
            .and(warp::header::optional("api-key"))
            .and(warp::header::optional("content-type"))
            .and(with_env(env.clone()))
            .and_then(handlers::interior_ref_list::create),
    );
    let delete_interior_ref_list_handler = warp::path("interior_ref_lists").and(
        warp::path::param()
            .and(warp::path::end())
            .and(warp::delete())
            .and(warp::header::optional("api-key"))
            .and(with_env(env.clone()))
            .and_then(handlers::interior_ref_list::delete),
    );
    let update_interior_ref_list_handler = warp::path("interior_ref_lists").and(
        warp::path::param()
            .and(warp::path::end())
            .and(warp::patch())
            .and(json_body::<InteriorRefList>())
            .and(warp::header::optional("api-key"))
            .and(warp::header::optional("content-type"))
            .and(with_env(env.clone()))
            .and_then(handlers::interior_ref_list::update),
    );
    let update_interior_ref_list_by_shop_id_handler = warp::path("shops").and(
        warp::path::param()
            .and(warp::path("interior_ref_list"))
            .and(warp::path::end())
            .and(warp::patch())
            .and(json_body::<InteriorRefList>())
            .and(warp::header::optional("api-key"))
            .and(warp::header::optional("content-type"))
            .and(with_env(env.clone()))
            .and_then(handlers::interior_ref_list::update_by_shop_id),
    );
    let list_interior_ref_lists_handler = warp::path("interior_ref_lists").and(
        warp::path::end()
            .and(warp::get())
            .and(warp::query::<ListParams>())
            .and(warp::header::optional("if-none-match"))
            .and(warp::header::optional("accept"))
            .and(with_env(env.clone()))
            .and_then(handlers::interior_ref_list::list),
    );
    let get_interior_ref_list_by_shop_id_handler = warp::path("shops").and(
        warp::path::param()
            .and(warp::path("interior_ref_list"))
            .and(warp::path::end())
            .and(warp::get())
            .and(warp::header::optional("if-none-match"))
            .and(warp::header::optional("accept"))
            .and(with_env(env.clone()))
            .and_then(handlers::interior_ref_list::get_by_shop_id),
    );
    let get_merchandise_list_handler = warp::path("merchandise_lists").and(
        warp::path::param()
            .and(warp::path::end())
            .and(warp::get())
            .and(warp::header::optional("if-none-match"))
            .and(warp::header::optional("accept"))
            .and(with_env(env.clone()))
            .and_then(handlers::merchandise_list::get),
    );
    let create_merchandise_list_handler = warp::path("merchandise_lists").and(
        warp::path::end()
            .and(warp::post())
            .and(json_body::<MerchandiseList>())
            .and(warp::header::optional("api-key"))
            .and(warp::header::optional("content-type"))
            .and(with_env(env.clone()))
            .and_then(handlers::merchandise_list::create),
    );
    let delete_merchandise_list_handler = warp::path("merchandise_lists").and(
        warp::path::param()
            .and(warp::path::end())
            .and(warp::delete())
            .and(warp::header::optional("api-key"))
            .and(with_env(env.clone()))
            .and_then(handlers::merchandise_list::delete),
    );
    let update_merchandise_list_handler = warp::path("merchandise_lists").and(
        warp::path::param()
            .and(warp::path::end())
            .and(warp::patch())
            .and(json_body::<MerchandiseList>())
            .and(warp::header::optional("api-key"))
            .and(warp::header::optional("content-type"))
            .and(with_env(env.clone()))
            .and_then(handlers::merchandise_list::update),
    );
    let update_merchandise_list_by_shop_id_handler = warp::path("shops").and(
        warp::path::param()
            .and(warp::path("merchandise_list"))
            .and(warp::path::end())
            .and(warp::patch())
            .and(json_body::<MerchandiseList>())
            .and(warp::header::optional("api-key"))
            .and(warp::header::optional("content-type"))
            .and(with_env(env.clone()))
            .and_then(handlers::merchandise_list::update_by_shop_id),
    );
    let list_merchandise_lists_handler = warp::path("merchandise_lists").and(
        warp::path::end()
            .and(warp::get())
            .and(warp::query::<ListParams>())
            .and(warp::header::optional("if-none-match"))
            .and(warp::header::optional("accept"))
            .and(with_env(env.clone()))
            .and_then(handlers::merchandise_list::list),
    );
    let get_merchandise_list_by_shop_id_handler = warp::path("shops").and(
        warp::path::param()
            .and(warp::path("merchandise_list"))
            .and(warp::path::end())
            .and(warp::get())
            .and(warp::header::optional("if-none-match"))
            .and(warp::header::optional("accept"))
            .and(with_env(env.clone()))
            .and_then(handlers::merchandise_list::get_by_shop_id),
    );
    let get_transaction_handler = warp::path("transactions").and(
        warp::path::param()
            .and(warp::path::end())
            .and(warp::get())
            .and(warp::header::optional("if-none-match"))
            .and(with_env(env.clone()))
            .and_then(handlers::transaction::get),
    );
    let create_transaction_handler = warp::path("transactions").and(
        warp::path::end()
            .and(warp::post())
            .and(json_body::<Transaction>())
            .and(warp::header::optional("api-key"))
            .and(with_env(env.clone()))
            .and_then(handlers::transaction::create),
    );
    let delete_transaction_handler = warp::path("transactions").and(
        warp::path::param()
            .and(warp::path::end())
            .and(warp::delete())
            .and(warp::header::optional("api-key"))
            .and(with_env(env.clone()))
            .and_then(handlers::transaction::delete),
    );
    let list_transactions_handler = warp::path("transactions").and(
        warp::path::end()
            .and(warp::get())
            .and(warp::query::<ListParams>())
            .and(warp::header::optional("if-none-match"))
            .and(with_env(env.clone()))
            .and_then(handlers::transaction::list),
    );
    let list_transactions_by_shop_id_handler = warp::path("shops").and(
        warp::path::param()
            .and(warp::path("transactions"))
            .and(warp::path::end())
            .and(warp::get())
            .and(warp::query::<ListParams>())
            .and(warp::header::optional("if-none-match"))
            .and(with_env(env.clone()))
            .and_then(handlers::transaction::list_by_shop_id),
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
            list_transactions_by_shop_id_handler,
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
            get_transaction_handler,
            delete_transaction_handler,
            create_transaction_handler,
            list_transactions_handler,
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
