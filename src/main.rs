#[macro_use]
extern crate log;

use anyhow::Result;
use clap::Clap;
use dotenv::dotenv;
use hyper::server::Server;
use listenfd::ListenFd;
use serde::Serialize;
use sqlx::postgres::PgPool;
use std::convert::Infallible;
use std::env;
use url::Url;
use warp::Filter;

mod db;
mod filters;
mod handlers;
mod models;
mod problem;

#[derive(Clap)]
#[clap(version = "0.1.0", author = "Tyler Hallada <tyler@hallada.net>")]
struct Opts {
    #[clap(short, long)]
    migrate: bool,
}

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

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "shopkeeper=info");
    }
    pretty_env_logger::init();
    let opts: Opts = Opts::parse();

    if opts.migrate {
        info!("going to migrate now!");
        db::migrate().await;
        return Ok(());
    }

    let host = env::var("HOST").expect("`HOST` environment variable not defined");
    let host_url = Url::parse(&host).expect("Cannot parse URL from `HOST` environment variable");
    let api_url = host_url.join("/api/v1")?;
    let env = Environment::new(api_url).await?;

    let base = warp::path("api").and(warp::path("v1"));
    let get_shop = filters::get_shop(env.clone());
    let create_shop = filters::create_shop(env.clone());
    let list_shops = filters::list_shops(env.clone());
    let get_owner = filters::get_owner(env.clone());
    let create_owner = filters::create_owner(env.clone());
    let list_owners = filters::list_owners(env.clone());
    let get_interior_ref = filters::get_interior_ref(env.clone());
    let create_interior_ref = filters::create_interior_ref(env.clone());
    let list_interior_refs = filters::list_interior_refs(env.clone());
    let bulk_create_interior_refs = filters::bulk_create_interior_refs(env.clone());
    let routes = base
        .and(
            create_shop
                .or(get_shop)
                .or(list_shops)
                .or(create_owner)
                .or(get_owner)
                .or(list_owners)
                .or(create_interior_ref)
                .or(get_interior_ref)
                .or(list_interior_refs)
                .or(bulk_create_interior_refs),
        )
        .recover(problem::unpack_problem)
        .with(warp::compression::gzip())
        .with(warp::log("shopkeeper"));

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
