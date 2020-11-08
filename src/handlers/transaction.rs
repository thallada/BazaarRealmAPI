use anyhow::{anyhow, Result};
use http::StatusCode;
use mime::Mime;
use uuid::Uuid;
use warp::reply::{with_header, with_status};
use warp::{Rejection, Reply};

use crate::caches::{Cache, CachedResponse, CACHES};
use crate::models::{ListParams, MerchandiseList, Transaction};
use crate::problem::reject_anyhow;
use crate::Environment;

use super::{authenticate, check_etag, AcceptHeader, Bincode, DataReply, ETagReply, Json};

pub async fn get(
    id: i32,
    etag: Option<String>,
    accept: Option<AcceptHeader>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    async fn get<T: DataReply>(
        id: i32,
        etag: Option<String>,
        env: Environment,
        cache: &'static Cache<i32, CachedResponse>,
    ) -> Result<Box<dyn Reply>, Rejection> {
        let response = cache
            .get_response(id, || async {
                let transaction = Transaction::get(&env.db, id).await?;
                let reply = T::from_serializable(&transaction)?;
                let reply = with_status(reply, StatusCode::OK);
                Ok(reply)
            })
            .await?;
        Ok(Box::new(check_etag(etag, response)))
    }

    match accept {
        Some(accept) if accept.accepts_bincode() => {
            get::<ETagReply<Bincode>>(id, etag, env, &CACHES.transaction_bin).await
        }
        _ => get::<ETagReply<Json>>(id, etag, env, &CACHES.transaction).await,
    }
}

pub async fn list(
    list_params: ListParams,
    etag: Option<String>,
    accept: Option<AcceptHeader>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    async fn get<T: DataReply>(
        list_params: ListParams,
        etag: Option<String>,
        env: Environment,
        cache: &'static Cache<ListParams, CachedResponse>,
    ) -> Result<Box<dyn Reply>, Rejection> {
        let response = cache
            .get_response(list_params.clone(), || async {
                let transactions = Transaction::list(&env.db, &list_params).await?;
                let reply = T::from_serializable(&transactions)?;
                let reply = with_status(reply, StatusCode::OK);
                Ok(reply)
            })
            .await?;
        Ok(Box::new(check_etag(etag, response)))
    }

    match accept {
        Some(accept) if accept.accepts_bincode() => {
            get::<ETagReply<Bincode>>(list_params, etag, env, &CACHES.list_transactions_bin).await
        }
        _ => get::<ETagReply<Json>>(list_params, etag, env, &CACHES.list_transactions).await,
    }
}

pub async fn list_by_shop_id(
    shop_id: i32,
    list_params: ListParams,
    etag: Option<String>,
    accept: Option<AcceptHeader>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    async fn get<T: DataReply>(
        shop_id: i32,
        list_params: ListParams,
        etag: Option<String>,
        env: Environment,
        cache: &'static Cache<(i32, ListParams), CachedResponse>,
    ) -> Result<Box<dyn Reply>, Rejection> {
        let response = cache
            .get_response((shop_id, list_params.clone()), || async {
                let transactions =
                    Transaction::list_by_shop_id(&env.db, shop_id, &list_params).await?;
                let reply = T::from_serializable(&transactions)?;
                let reply = with_status(reply, StatusCode::OK);
                Ok(reply)
            })
            .await?;
        Ok(Box::new(check_etag(etag, response)))
    }

    match accept {
        Some(accept) if accept.accepts_bincode() => {
            get::<ETagReply<Bincode>>(
                shop_id,
                list_params,
                etag,
                env,
                &CACHES.list_transactions_by_shop_id_bin,
            )
            .await
        }
        _ => {
            get::<ETagReply<Json>>(
                shop_id,
                list_params,
                etag,
                env,
                &CACHES.list_transactions_by_shop_id,
            )
            .await
        }
    }
}

pub async fn create(
    transaction: Transaction,
    api_key: Option<Uuid>,
    content_type: Option<Mime>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    async fn create<'a, T: DataReply + 'a>(
        transaction: Transaction,
        api_key: Option<Uuid>,
        env: Environment,
    ) -> Result<Box<dyn Reply + 'a>, Rejection> {
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
        let reply = T::from_serializable(&saved_transaction).map_err(reject_anyhow)?;
        let reply = with_header(reply, "Location", url.as_str());
        let reply = with_status(reply, StatusCode::CREATED);
        tokio::spawn(async move {
            // TODO: will this make these caches effectively useless?
            let merch_id = updated_merchandise_list
                .id
                .expect("saved merchandise_list has no id");
            CACHES.merchandise_list.delete_response(merch_id).await;
            CACHES.merchandise_list_bin.delete_response(merch_id).await;
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
        Ok(Box::new(reply))
    }

    match content_type {
        Some(content_type) if content_type == mime::APPLICATION_OCTET_STREAM => {
            create::<ETagReply<Bincode>>(transaction, api_key, env).await
        }
        _ => create::<ETagReply<Json>>(transaction, api_key, env).await,
    }
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
