use anyhow::{anyhow, Result};
use http::StatusCode;
use hyper::body::Bytes;
use mime::Mime;
use uuid::Uuid;
use warp::reply::{with_header, with_status};
use warp::{Rejection, Reply};

use crate::caches::{CachedResponse, CACHES};
use crate::models::{ListParams, MerchandiseList, PostedTransaction, Transaction};
use crate::problem::reject_anyhow;
use crate::Environment;

use super::{
    authenticate, check_etag, AcceptHeader, Bincode, ContentType, DataReply, DeserializedBody,
    ETagReply, Json, TypedCache,
};

pub async fn get(
    id: i32,
    etag: Option<String>,
    accept: Option<AcceptHeader>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let TypedCache {
        content_type,
        cache,
    } = TypedCache::<i32, CachedResponse>::pick_cache(
        accept,
        &CACHES.transaction_bin,
        &CACHES.transaction,
    );
    let response = cache
        .get_response(id, || async {
            let transaction = Transaction::get(&env.db, id).await?;
            let reply: Box<dyn Reply> = match content_type {
                ContentType::Bincode => {
                    Box::new(ETagReply::<Bincode>::from_serializable(&transaction)?)
                }
                ContentType::Json => Box::new(ETagReply::<Json>::from_serializable(&transaction)?),
            };
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await?;
    Ok(check_etag(etag, response))
}

pub async fn list(
    list_params: ListParams,
    etag: Option<String>,
    accept: Option<AcceptHeader>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let TypedCache {
        content_type,
        cache,
    } = TypedCache::<ListParams, CachedResponse>::pick_cache(
        accept,
        &CACHES.list_transactions_bin,
        &CACHES.list_transactions,
    );
    let response = cache
        .get_response(list_params.clone(), || async {
            let transactions = Transaction::list(&env.db, &list_params).await?;
            let reply: Box<dyn Reply> = match content_type {
                ContentType::Bincode => {
                    Box::new(ETagReply::<Bincode>::from_serializable(&transactions)?)
                }
                ContentType::Json => Box::new(ETagReply::<Json>::from_serializable(&transactions)?),
            };
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
    accept: Option<AcceptHeader>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let TypedCache {
        content_type,
        cache,
    } = TypedCache::<(i32, ListParams), CachedResponse>::pick_cache(
        accept,
        &CACHES.list_transactions_by_shop_id_bin,
        &CACHES.list_transactions_by_shop_id,
    );
    let response = cache
        .get_response((shop_id, list_params.clone()), || async {
            let transactions = Transaction::list_by_shop_id(&env.db, shop_id, &list_params).await?;
            let reply: Box<dyn Reply> = match content_type {
                ContentType::Bincode => {
                    Box::new(ETagReply::<Bincode>::from_serializable(&transactions)?)
                }
                ContentType::Json => Box::new(ETagReply::<Json>::from_serializable(&transactions)?),
            };
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await?;
    Ok(check_etag(etag, response))
}

pub async fn create(
    bytes: Bytes,
    api_key: Option<Uuid>,
    content_type: Option<Mime>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let DeserializedBody {
        body: mut transaction,
        content_type,
    } = DeserializedBody::<PostedTransaction>::from_bytes(bytes, content_type)
        .map_err(reject_anyhow)?;
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    transaction.owner_id = Some(owner_id);
    let mut tx = env
        .db
        .begin()
        .await
        .map_err(|error| reject_anyhow(anyhow!(error)))?;
    let saved_transaction = Transaction::create(transaction, &mut tx)
        .await
        .map_err(reject_anyhow)?;
    let quantity_delta = match saved_transaction.is_sell {
        true => saved_transaction.quantity,
        false => saved_transaction.quantity * -1,
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
    let reply: Box<dyn Reply> = match content_type {
        ContentType::Bincode => Box::new(
            ETagReply::<Bincode>::from_serializable(&saved_transaction).map_err(reject_anyhow)?,
        ),
        ContentType::Json => Box::new(
            ETagReply::<Json>::from_serializable(&saved_transaction).map_err(reject_anyhow)?,
        ),
    };
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    tokio::spawn(async move {
        // TODO: will this make these caches effectively useless?
        CACHES
            .merchandise_list
            .delete_response(updated_merchandise_list.id)
            .await;
        CACHES
            .merchandise_list_bin
            .delete_response(updated_merchandise_list.id)
            .await;
        CACHES
            .merchandise_list_by_shop_id
            .delete_response(updated_merchandise_list.shop_id)
            .await;
        CACHES
            .merchandise_list_by_shop_id_bin
            .delete_response(updated_merchandise_list.shop_id)
            .await;
        CACHES.list_transactions.clear().await;
        CACHES.list_transactions_bin.clear().await;
        CACHES.list_transactions_by_shop_id.clear().await;
        CACHES.list_transactions_by_shop_id_bin.clear().await;
        CACHES.list_merchandise_lists.clear().await;
        CACHES.list_merchandise_lists_bin.clear().await;
    });
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
    tokio::spawn(async move {
        CACHES.transaction.delete_response(id).await;
        CACHES.transaction_bin.delete_response(id).await;
        CACHES.list_transactions.clear().await;
        CACHES.list_transactions_bin.clear().await;
        CACHES.list_transactions_by_shop_id.clear().await;
        CACHES.list_transactions_by_shop_id_bin.clear().await;
    });
    Ok(StatusCode::NO_CONTENT)
}
