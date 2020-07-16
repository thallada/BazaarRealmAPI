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

    info!("warp speed ahead!");

    let home = warp::path!("api" / "v1").map(|| "Shopkeeper home page");
    let get_shop = filters::get_shop(env.clone());
    let create_shop = filters::create_shop(env.clone());
    let get_owner = filters::get_owner(env.clone());
    let create_owner = filters::create_owner(env.clone());
    let routes = create_shop
        .or(get_shop)
        .or(create_owner)
        .or(get_owner)
        .or(home)
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

mod filters {
    use serde::de::DeserializeOwned;
    use std::convert::Infallible;
    use warp::{Filter, Rejection, Reply};

    use super::handlers;
    use super::models::{Owner, Shop};
    use super::Environment;

    pub fn get_shop(
        env: Environment,
    ) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        warp::path!("shops" / i32)
            .and(warp::get())
            .and(with_env(env))
            .and_then(handlers::get_shop)
    }

    pub fn create_shop(
        env: Environment,
    ) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        warp::path("shops")
            .and(warp::post())
            .and(json_body::<Shop>())
            .and(with_env(env))
            .and_then(handlers::create_shop)
    }

    pub fn get_owner(
        env: Environment,
    ) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        warp::path!("owners" / i32)
            .and(warp::get())
            .and(with_env(env))
            .and_then(handlers::get_owner)
    }

    pub fn create_owner(
        env: Environment,
    ) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        warp::path("owners")
            .and(warp::post())
            .and(json_body::<Owner>())
            .and(warp::addr::remote())
            .and(with_env(env))
            .and_then(handlers::create_owner)
    }

    fn with_env(
        env: Environment,
    ) -> impl Filter<Extract = (Environment,), Error = Infallible> + Clone {
        warp::any().map(move || env.clone())
    }

    fn json_body<T>() -> impl Filter<Extract = (T,), Error = warp::Rejection> + Clone
    where
        T: Send + DeserializeOwned,
    {
        warp::body::content_length_limit(1024 * 16).and(warp::body::json())
    }
}

mod handlers {
    use ipnetwork::IpNetwork;
    use std::net::SocketAddr;
    use warp::http::StatusCode;
    use warp::reply::{json, with_header, with_status};
    use warp::{Rejection, Reply};

    use super::models::{Owner, Shop};
    use super::problem::reject_anyhow;
    use super::Environment;

    pub async fn get_shop(id: i32, env: Environment) -> Result<impl Reply, Rejection> {
        let shop = Shop::get(&env.db, id).await.map_err(reject_anyhow)?;
        let reply = json(&shop);
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
}

mod models {
    use anyhow::{anyhow, Result};
    use chrono::prelude::*;
    use ipnetwork::IpNetwork;
    use serde::{Deserialize, Serialize};
    use sqlx::postgres::PgPool;
    use url::Url;
    use uuid::Uuid;

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct Shop {
        pub id: Option<i32>,
        pub name: String,
        pub owner_id: i32,
        pub description: String,
        pub is_not_sell_buy: bool,
        pub sell_buy_list_id: i32,
        pub vendor_id: i32,
        pub vendor_gold: i32,
        pub created_at: Option<NaiveDateTime>,
        pub updated_at: Option<NaiveDateTime>,
    }

    impl Shop {
        pub fn url(&self, api_url: &Url) -> Result<Url> {
            if let Some(id) = self.id {
                Ok(api_url.join(&format!("/shops/{}", id))?)
            } else {
                Err(anyhow!("Cannot get URL for shop with no id"))
            }
        }

        pub async fn get(db: &PgPool, id: i32) -> Result<Self> {
            let timer = std::time::Instant::now();
            let result = sqlx::query_as!(Self, "SELECT * FROM shops WHERE id = $1", id)
                .fetch_one(db)
                .await?;
            let elapsed = timer.elapsed();
            debug!("SELECT * FROM shops ... {:.3?}", elapsed);
            Ok(result)
        }

        pub async fn save(self, db: &PgPool) -> Result<Self> {
            let timer = std::time::Instant::now();
            let result = sqlx::query_as!(
                Self,
                "INSERT INTO shops
                (name, owner_id, description, is_not_sell_buy, sell_buy_list_id, vendor_id,
                 vendor_gold, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, now(), now())
                RETURNING *",
                self.name,
                self.owner_id,
                self.description,
                self.is_not_sell_buy,
                self.sell_buy_list_id,
                self.vendor_id,
                self.vendor_gold,
            )
            .fetch_one(db)
            .await?;
            let elapsed = timer.elapsed();
            debug!("INSERT INTO shops ... {:.3?}", elapsed);
            Ok(result)
        }
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct Owner {
        pub id: Option<i32>,
        pub name: String,
        pub api_key: Uuid,
        pub ip_address: Option<IpNetwork>,
        pub mod_version: String,
        pub created_at: Option<NaiveDateTime>,
        pub updated_at: Option<NaiveDateTime>,
    }

    impl Owner {
        pub fn url(&self, api_url: &Url) -> Result<Url> {
            if let Some(id) = self.id {
                Ok(api_url.join(&format!("/owners/{}", id))?)
            } else {
                Err(anyhow!("Cannot get URL for owner with no id"))
            }
        }

        pub async fn get(db: &PgPool, id: i32) -> Result<Self> {
            let timer = std::time::Instant::now();
            let result = sqlx::query_as!(Self, "SELECT * FROM owners WHERE id = $1", id)
                .fetch_one(db)
                .await?;
            let elapsed = timer.elapsed();
            debug!("SELECT * FROM owners ... {:.3?}", elapsed);
            Ok(result)
        }

        pub async fn save(self, db: &PgPool) -> Result<Self> {
            let timer = std::time::Instant::now();
            let result = sqlx::query_as!(
                Self,
                "INSERT INTO owners
                (name, api_key, ip_address, mod_version, created_at, updated_at)
                VALUES ($1, $2, $3, $4, now(), now())
                RETURNING *",
                self.name,
                self.api_key,
                self.ip_address,
                self.mod_version,
            )
            .fetch_one(db)
            .await?;
            let elapsed = timer.elapsed();
            debug!("INSERT INTO owners ... {:.3?}", elapsed);
            Ok(result)
        }
    }
}

mod problem {
    use http_api_problem::HttpApiProblem;
    use warp::http::StatusCode;
    use warp::{reject, Rejection, Reply};

    pub fn from_anyhow(error: anyhow::Error) -> HttpApiProblem {
        let error = match error.downcast::<HttpApiProblem>() {
            Ok(problem) => return problem,
            Err(error) => error,
        };

        if let Some(sqlx_error) = error.downcast_ref::<sqlx::error::Error>() {
            match sqlx_error {
                sqlx::error::Error::RowNotFound => {
                    return HttpApiProblem::with_title_and_type_from_status(StatusCode::NOT_FOUND)
                }
                sqlx::error::Error::Database(db_error) => {
                    error!(
                        "Database error: {}. {}",
                        db_error.message(),
                        db_error.details().unwrap_or("")
                    );
                    if let Some(code) = db_error.code() {
                        if let Some(constraint) = db_error.constraint_name() {
                            if code == "23503" && constraint == "shops_owner_id_fkey" {
                                // foreign_key_violation
                                return HttpApiProblem::with_title_and_type_from_status(
                                    StatusCode::BAD_REQUEST,
                                )
                                .set_detail("Owner does not exist");
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        error!("Recovering unhandled error: {:?}", error);
        // TODO: this leaks internal info, should not stringify error
        HttpApiProblem::new(format!("Internal Server Error: {:?}", error))
            .set_status(StatusCode::INTERNAL_SERVER_ERROR)
    }

    pub async fn unpack_problem(rejection: Rejection) -> Result<impl Reply, Rejection> {
        if let Some(problem) = rejection.find::<HttpApiProblem>() {
            let code = problem.status.unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

            let reply = warp::reply::json(problem);
            let reply = warp::reply::with_status(reply, code);
            let reply = warp::reply::with_header(
                reply,
                warp::http::header::CONTENT_TYPE,
                http_api_problem::PROBLEM_JSON_MEDIA_TYPE,
            );

            return Ok(reply);
        }

        Err(rejection)
    }

    pub fn reject_anyhow(error: anyhow::Error) -> Rejection {
        reject::custom(from_anyhow(error))
    }
}
