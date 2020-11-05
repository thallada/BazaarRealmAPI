use anyhow::{anyhow, Result};
use http::StatusCode;
use uuid::Uuid;
use warp::reply::{with_header, with_status};
use warp::{Rejection, Reply};

use crate::models::{ListParams, MerchandiseList, Transaction};
use crate::problem::reject_anyhow;
use crate::Environment;

use super::{authenticate, check_etag, JsonWithETag};

pub async fn get(id: i32, etag: Option<String>, env: Environment) -> Result<impl Reply, Rejection> {
    let response = env
        .caches
        .transaction
        .get_response(id, || async {
            let transaction = Transaction::get(&env.db, id).await?;
            let reply = JsonWithETag::from_serializable(&transaction)?;
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await?;
    Ok(check_etag(etag, response))
}

pub async fn list(
    list_params: ListParams,
    etag: Option<String>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let response = env
        .caches
        .list_transactions
        .get_response(list_params.clone(), || async {
            let transactions = Transaction::list(&env.db, &list_params).await?;
            let reply = JsonWithETag::from_serializable(&transactions)?;
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await?;
    Ok(check_etag(etag, response))
}

pub async fn list_by_shop_id(
    shop_id: i32,
    list_params: ListParams,
    etag: Option<String>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let response = env
        .caches
        .list_transactions_by_shop_id
        .get_response((shop_id, list_params.clone()), || async {
            let transactions = Transaction::list_by_shop_id(&env.db, shop_id, &list_params).await?;
            let reply = JsonWithETag::from_serializable(&transactions)?;
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await?;
    Ok(check_etag(etag, response))
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
    let mut tx = env
        .db
        .begin()
        .await
        .map_err(|error| reject_anyhow(anyhow!(error)))?;
    let saved_transaction = transaction_with_owner_id
        .create(&mut tx)
        .await
        .map_err(reject_anyhow)?;
    let quantity_delta = match transaction.is_sell {
        true => transaction.quantity,
        false => transaction.quantity * -1,
    };
    let updated_merchandise_list = MerchandiseList::update_merchandise_quantity(
        &mut tx,
        saved_transaction.shop_id,
        &(saved_transaction.mod_name),
        saved_transaction.local_form_id,
        &(saved_transaction.name),
        saved_transaction.form_type,
        saved_transaction.is_food,
        saved_transaction.price,
        quantity_delta,
    )
    .await
    .map_err(reject_anyhow)?;
    tx.commit()
        .await
        .map_err(|error| reject_anyhow(anyhow!(error)))?;
    let url = saved_transaction.url(&env.api_url).map_err(reject_anyhow)?;
    let reply = JsonWithETag::from_serializable(&saved_transaction).map_err(reject_anyhow)?;
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    // TODO: will this make these caches effectively useless?
    env.caches.list_transactions.clear().await;
    env.caches.list_transactions_by_shop_id.clear().await;
    env.caches
        .merchandise_list
        .delete_response(
            updated_merchandise_list
                .id
                .expect("saved merchandise_list has no id"),
        )
        .await;
    env.caches
        .merchandise_list_by_shop_id
        .delete_response(updated_merchandise_list.shop_id)
        .await;
    env.caches.list_merchandise_lists.clear().await;
    Ok(reply)
}

pub async fn delete(
    id: i32,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    Transaction::delete(&env.db, owner_id, id)
        .await
        .map_err(reject_anyhow)?;
    env.caches.transaction.delete_response(id).await;
    env.caches.list_transactions.clear().await;
    env.caches.list_transactions_by_shop_id.clear().await;
    Ok(StatusCode::NO_CONTENT)
}
