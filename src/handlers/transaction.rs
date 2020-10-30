use anyhow::Result;
use http::StatusCode;
use uuid::Uuid;
use warp::reply::{json, with_header, with_status};
use warp::{Rejection, Reply};

use crate::models::{ListParams, Model, Transaction};
use crate::problem::reject_anyhow;
use crate::Environment;

use super::authenticate;

pub async fn get(id: i32, env: Environment) -> Result<impl Reply, Rejection> {
    env.caches
        .transaction
        .get_response(id, || async {
            let transaction = Transaction::get(&env.db, id).await?;
            let reply = json(&transaction);
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await
}

pub async fn list(list_params: ListParams, env: Environment) -> Result<impl Reply, Rejection> {
    env.caches
        .list_transactions
        .get_response(list_params.clone(), || async {
            let transactions = Transaction::list(&env.db, &list_params).await?;
            let reply = json(&transactions);
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await
}

pub async fn list_by_shop_id(
    shop_id: i32,
    list_params: ListParams,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    env.caches
        .list_transactions_by_shop_id
        .get_response((shop_id, list_params.clone()), || async {
            let transactions = Transaction::list_by_shop_id(&env.db, shop_id, &list_params).await?;
            let reply = json(&transactions);
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await
}

pub async fn create(
    transaction: Transaction,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    let transaction_with_owner_id = Transaction {
        owner_id: Some(owner_id),
        ..transaction
    };
    let saved_transaction = transaction_with_owner_id
        .create(&env.db)
        .await
        .map_err(reject_anyhow)?;
    let url = saved_transaction.url(&env.api_url).map_err(reject_anyhow)?;
    let reply = json(&saved_transaction);
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    // TODO: will this make these caches effectively useless?
    env.caches.list_transactions.clear().await;
    env.caches.list_transactions_by_shop_id.clear().await;
    Ok(reply)
}

pub async fn delete(
    id: i32,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    let transaction = Transaction::get(&env.db, id).await.map_err(reject_anyhow)?;
    Transaction::delete(&env.db, owner_id, id)
        .await
        .map_err(reject_anyhow)?;
    env.caches.transaction.delete_response(id).await;
    env.caches.list_transactions.clear().await;
    env.caches.list_transactions_by_shop_id.clear().await;
    Ok(StatusCode::NO_CONTENT)
}
