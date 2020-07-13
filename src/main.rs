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
}

impl Environment {
    async fn new() -> Result<Environment> {
        Ok(Environment {
            db: PgPool::builder()
                .max_size(5)
                .build(&env::var("DATABASE_URL")?)
                .await?,
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

    let env = Environment::new().await?;

    info!("warp speed ahead!");

    // TODO: need to put everything under /api/v1/
    let home = warp::path::end().map(|| "Shopkeeper home page");
    let view_shop = filters::view_shop(env.clone());
    let create_shop = filters::create_shop(env.clone());
    let routes = create_shop
        .or(view_shop)
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
    use std::convert::Infallible;
    use warp::{Filter, Rejection, Reply};

    use super::handlers;
    use super::models::Shop;
    use super::Environment;

    pub fn view_shop(
        env: Environment,
    ) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        warp::path("shops")
            .and(with_env(env))
            .and(warp::get())
            .and(warp::path::param())
            .and_then(handlers::get_shop)
    }

    pub fn create_shop(
        env: Environment,
    ) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        warp::path("shops")
            .and(with_env(env))
            .and(warp::post())
            .and(json_body())
            .and_then(handlers::create_shop)
    }

    fn with_env(
        env: Environment,
    ) -> impl Filter<Extract = (Environment,), Error = Infallible> + Clone {
        warp::any().map(move || env.clone())
    }

    fn json_body() -> impl Filter<Extract = (Shop,), Error = warp::Rejection> + Clone {
        warp::body::content_length_limit(1024 * 16).and(warp::body::json())
    }
}

mod handlers {
    use warp::{Rejection, Reply};

    use super::models::Shop;
    use super::problem::reject_anyhow;
    use super::Environment;

    pub async fn get_shop(env: Environment, id: i32) -> Result<impl Reply, Rejection> {
        dbg!(id);
        let shop = Shop::get(&env.db, id).await.map_err(reject_anyhow)?;
        return Ok(format!("Shop {}: {}.", id, shop.name));
    }

    pub async fn create_shop(env: Environment, shop: Shop) -> Result<impl Reply, Rejection> {
        dbg!(&shop);
        shop.create(&env.db).await.map_err(reject_anyhow)?;
        return Ok(format!("Shop {}: {}.", "unknown", &shop.name));
    }
}

mod models {
    use anyhow::Result;
    use chrono::prelude::*;
    use http_api_problem::HttpApiProblem;
    use serde::{Deserialize, Serialize};
    use sqlx::postgres::PgPool;
    use warp::http::StatusCode;

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
        pub async fn get(db: &PgPool, id: i32) -> Result<Shop> {
            let timer = std::time::Instant::now();
            let result = sqlx::query_as!(Self, "SELECT * FROM shops WHERE id = $1", id)
                .fetch_one(db)
                .await?;
            let elapsed = timer.elapsed();
            dbg!(elapsed);
            info!("SELECT * FROM shops ... | {:.3?} elapsed", elapsed);
            Ok(result)
        }

        pub async fn create(&self, db: &PgPool) -> Result<()> {
            let timer = std::time::Instant::now();
            let result = sqlx::query!(
                "INSERT INTO shops
                (name, owner_id, description, is_not_sell_buy, sell_buy_list_id, vendor_id,
                 vendor_gold, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, now(), now())",
                self.name,
                self.owner_id,
                self.description,
                self.is_not_sell_buy,
                self.sell_buy_list_id,
                self.vendor_id,
                self.vendor_gold,
            )
            .execute(db)
            .await
            .map_err(|error| {
                if let sqlx::error::Error::Database(db_error) = &error {
                    if db_error
                        .message()
                        .contains("violates foreign key constraint \"shops_owner_id_fkey\"")
                    {
                        return anyhow::Error::new(
                            HttpApiProblem::with_title_and_type_from_status(
                                StatusCode::BAD_REQUEST,
                            )
                            .set_detail(format!("Owner with id: {} does not exist", self.owner_id)),
                        );
                    }
                }
                anyhow::Error::new(error)
            })?;
            dbg!(result);
            let elapsed = timer.elapsed();
            dbg!(elapsed);
            info!("INSERT INTO shops ... | {:.3?} elapsed", elapsed);
            Ok(())
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
                }
                _ => {}
            }
        }

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
