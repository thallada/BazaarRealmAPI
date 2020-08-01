use anyhow::Result;
use clap::Clap;
use dotenv::dotenv;
use hyper::server::Server;
use listenfd::ListenFd;
use serde::Serialize;
use sqlx::postgres::PgPool;
use std::convert::Infallible;
use std::env;
use tracing::info;
use tracing_subscriber::fmt::format::FmtSpan;
use url::Url;
use warp::Filter;

mod caches;
mod db;
mod filters;
mod handlers;
mod models;
mod problem;

use caches::Caches;

#[derive(Clap)]
#[clap(version = "0.1.0", author = "Tyler Hallada <tyler@hallada.net>")]
struct Opts {
    #[clap(short, long)]
    migrate: bool,
}

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

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let env_log_filter =
        env::var("RUST_LOG").unwrap_or_else(|_| "warp=info,shopkeeper=info".to_owned());
    tracing_subscriber::fmt()
        .with_env_filter(env_log_filter)
        .with_span_events(FmtSpan::CLOSE)
        .init();
    let opts: Opts = Opts::parse();

    if opts.migrate {
        info!("going to migrate now!");
        db::migrate().await;
        return Ok(());
    }

    let host = env::var("HOST").expect("`HOST` environment variable not defined");
    let host_url = Url::parse(&host).expect("Cannot parse URL from `HOST` environment variable");
    let api_url = host_url.join("/api/v1/")?;
    let env = Environment::new(api_url).await?;

    let base = warp::path("api").and(warp::path("v1"));
    let routes = base
        .and(
            filters::shops(env.clone())
                .or(filters::owners(env.clone()))
                .or(filters::interior_ref_lists(env.clone())),
        )
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
        Server::bind(&([0, 0, 0, 0], 3030).into())
    };

    // warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
    server.serve(make_svc).await?;
    Ok(())
}
