use anyhow::{anyhow, Result};
use http::StatusCode;
use ipnetwork::IpNetwork;
use sqlx::types::Json;
use std::net::SocketAddr;
use tracing::instrument;
use uuid::Uuid;
use warp::reply::{json, with_header, with_status};
use warp::{Rejection, Reply};

use super::models::{
    InteriorRefList, ListParams, MerchandiseList, MerchandiseParams, Model, Owner, Shop,
    UpdateableModel,
};
use super::problem::{reject_anyhow, unauthorized_no_api_key, unauthorized_no_owner};
use super::Environment;

#[instrument(level = "debug", skip(env, api_key))]
pub async fn authenticate(env: &Environment, api_key: Option<Uuid>) -> Result<i32> {
    if let Some(api_key) = api_key {
        env.caches
            .owner_ids_by_api_key
            .get(api_key, || async {
                Ok(
                    sqlx::query!("SELECT id FROM owners WHERE api_key = $1", api_key)
                        .fetch_one(&env.db)
                        .await
                        .map_err(|error| {
                            if let sqlx::Error::RowNotFound = error {
                                return unauthorized_no_owner();
                            }
                            anyhow!(error)
                        })?
                        .id,
                )
            })
            .await
    } else {
        Err(unauthorized_no_api_key())
    }
}

pub async fn get_shop(id: i32, env: Environment) -> Result<impl Reply, Rejection> {
    env.caches
        .shop
        .get_response(id, || async {
            let shop = Shop::get(&env.db, id).await?;
            let reply = json(&shop);
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await
}

pub async fn list_shops(
    list_params: ListParams,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    env.caches
        .list_shops
        .get_response(list_params.clone(), || async {
            let shops = Shop::list(&env.db, &list_params).await?;
            let reply = json(&shops);
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await
}

pub async fn create_shop(
    shop: Shop,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    let shop_with_owner_id = Shop {
        owner_id: Some(owner_id),
        ..shop
    };
    let saved_shop = shop_with_owner_id
        .create(&env.db)
        .await
        .map_err(reject_anyhow)?;

    // also save empty interior_ref_list and merchandise_list rows
    if let Some(shop_id) = saved_shop.id {
        let interior_ref_list = InteriorRefList {
            id: None,
            shop_id,
            owner_id: Some(owner_id),
            ref_list: Json::default(),
            created_at: None,
            updated_at: None,
        };
        interior_ref_list
            .create(&env.db)
            .await
            .map_err(reject_anyhow)?;
        let merchandise_list = MerchandiseList {
            id: None,
            shop_id,
            owner_id: Some(owner_id),
            form_list: Json::default(),
            created_at: None,
            updated_at: None,
        };
        merchandise_list
            .create(&env.db)
            .await
            .map_err(reject_anyhow)?;
    }

    let url = saved_shop.url(&env.api_url).map_err(reject_anyhow)?;
    let reply = json(&saved_shop);
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    env.caches.list_shops.clear().await;
    Ok(reply)
}

pub async fn update_shop(
    id: i32,
    shop: Shop,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    let shop_with_id_and_owner_id = if shop.owner_id.is_some() {
        // allows an owner to transfer ownership of shop to another owner
        Shop {
            id: Some(id),
            ..shop
        }
    } else {
        Shop {
            id: Some(id),
            owner_id: Some(owner_id),
            ..shop
        }
    };
    let updated_shop = shop_with_id_and_owner_id
        .update(&env.db, owner_id, id)
        .await
        .map_err(reject_anyhow)?;
    let url = updated_shop.url(&env.api_url).map_err(reject_anyhow)?;
    let reply = json(&updated_shop);
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    env.caches.shop.delete_response(id).await;
    env.caches.list_shops.clear().await;
    Ok(reply)
}

pub async fn delete_shop(
    id: i32,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    Shop::delete(&env.db, owner_id, id)
        .await
        .map_err(reject_anyhow)?;
    env.caches.shop.delete_response(id).await;
    env.caches.list_shops.clear().await;
    env.caches
        .interior_ref_list_by_shop_id
        .delete_response(id)
        .await;
    env.caches
        .merchandise_list_by_shop_id
        .delete_response(id)
        .await;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_owner(id: i32, env: Environment) -> Result<impl Reply, Rejection> {
    env.caches
        .owner
        .get_response(id, || async {
            let owner = Owner::get(&env.db, id).await?;
            let reply = json(&owner);
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await
}

pub async fn list_owners(
    list_params: ListParams,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    env.caches
        .list_owners
        .get_response(list_params.clone(), || async {
            let owners = Owner::list(&env.db, &list_params).await?;
            let reply = json(&owners);
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await
}

pub async fn create_owner(
    owner: Owner,
    remote_addr: Option<SocketAddr>,
    api_key: Option<Uuid>,
    real_ip: Option<IpNetwork>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    if let Some(api_key) = api_key {
        let owner_with_ip_and_key = match remote_addr {
            Some(addr) => Owner {
                api_key: Some(api_key),
                ip_address: Some(IpNetwork::from(addr.ip())),
                ..owner
            },
            None => Owner {
                api_key: Some(api_key),
                ip_address: real_ip,
                ..owner
            },
        };
        let saved_owner = owner_with_ip_and_key
            .create(&env.db)
            .await
            .map_err(reject_anyhow)?;
        let url = saved_owner.url(&env.api_url).map_err(reject_anyhow)?;
        let reply = json(&saved_owner);
        let reply = with_header(reply, "Location", url.as_str());
        let reply = with_status(reply, StatusCode::CREATED);
        env.caches.list_owners.clear().await;
        Ok(reply)
    } else {
        Err(reject_anyhow(unauthorized_no_api_key()))
    }
}

pub async fn update_owner(
    id: i32,
    owner: Owner,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    let owner_with_id = Owner {
        id: Some(id),
        ..owner
    };
    let updated_owner = owner_with_id
        .update(&env.db, owner_id, id)
        .await
        .map_err(reject_anyhow)?;
    let url = updated_owner.url(&env.api_url).map_err(reject_anyhow)?;
    let reply = json(&updated_owner);
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    env.caches.owner.delete_response(id).await;
    env.caches.list_owners.clear().await;
    Ok(reply)
}

pub async fn delete_owner(
    id: i32,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    Owner::delete(&env.db, owner_id, id)
        .await
        .map_err(reject_anyhow)?;
    env.caches.owner.delete_response(id).await;
    env.caches
        .owner_ids_by_api_key
        .delete(api_key.expect("api-key has been validated during authenticate"))
        .await;
    env.caches.list_owners.clear().await;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_interior_ref_list(id: i32, env: Environment) -> Result<impl Reply, Rejection> {
    env.caches
        .interior_ref_list
        .get_response(id, || async {
            let interior_ref_list = InteriorRefList::get(&env.db, id).await?;
            let reply = json(&interior_ref_list);
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await
}

pub async fn list_interior_ref_lists(
    list_params: ListParams,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    env.caches
        .list_interior_ref_lists
        .get_response(list_params.clone(), || async {
            let interior_ref_lists = InteriorRefList::list(&env.db, &list_params).await?;
            let reply = json(&interior_ref_lists);
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await
}

pub async fn create_interior_ref_list(
    interior_ref_list: InteriorRefList,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    let ref_list_with_owner_id = InteriorRefList {
        owner_id: Some(owner_id),
        ..interior_ref_list
    };
    let saved_interior_ref_list = ref_list_with_owner_id
        .create(&env.db)
        .await
        .map_err(reject_anyhow)?;
    let url = saved_interior_ref_list
        .url(&env.api_url)
        .map_err(reject_anyhow)?;
    let reply = json(&saved_interior_ref_list);
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    env.caches.list_interior_ref_lists.clear().await;
    env.caches
        .interior_ref_list_by_shop_id
        .delete_response(saved_interior_ref_list.shop_id)
        .await;
    Ok(reply)
}

pub async fn update_interior_ref_list(
    id: i32,
    interior_ref_list: InteriorRefList,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    let interior_ref_list_with_id_and_owner_id = if interior_ref_list.owner_id.is_some() {
        InteriorRefList {
            id: Some(id),
            ..interior_ref_list
        }
    } else {
        InteriorRefList {
            id: Some(id),
            owner_id: Some(owner_id),
            ..interior_ref_list
        }
    };
    let updated_interior_ref_list = interior_ref_list_with_id_and_owner_id
        .update(&env.db, owner_id, id)
        .await
        .map_err(reject_anyhow)?;
    let url = updated_interior_ref_list
        .url(&env.api_url)
        .map_err(reject_anyhow)?;
    let reply = json(&updated_interior_ref_list);
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    env.caches.interior_ref_list.delete_response(id).await;
    env.caches
        .interior_ref_list_by_shop_id
        .delete_response(updated_interior_ref_list.shop_id)
        .await;
    env.caches.list_interior_ref_lists.clear().await;
    Ok(reply)
}

pub async fn update_interior_ref_list_by_shop_id(
    shop_id: i32,
    interior_ref_list: InteriorRefList,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    let interior_ref_list_with_owner_id = InteriorRefList {
        owner_id: Some(owner_id),
        ..interior_ref_list
    };
    let updated_interior_ref_list = interior_ref_list_with_owner_id
        .update_by_shop_id(&env.db, owner_id, shop_id)
        .await
        .map_err(reject_anyhow)?;
    let url = updated_interior_ref_list
        .url(&env.api_url)
        .map_err(reject_anyhow)?;
    let reply = json(&updated_interior_ref_list);
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    env.caches
        .interior_ref_list
        .delete_response(
            updated_interior_ref_list
                .id
                .expect("saved interior_ref_list has no id"),
        )
        .await;
    env.caches
        .interior_ref_list_by_shop_id
        .delete_response(updated_interior_ref_list.shop_id)
        .await;
    env.caches.list_interior_ref_lists.clear().await;
    Ok(reply)
}

pub async fn delete_interior_ref_list(
    id: i32,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    let interior_ref_list = InteriorRefList::get(&env.db, id)
        .await
        .map_err(reject_anyhow)?;
    InteriorRefList::delete(&env.db, owner_id, id)
        .await
        .map_err(reject_anyhow)?;
    env.caches.interior_ref_list.delete_response(id).await;
    env.caches.list_interior_ref_lists.clear().await;
    env.caches
        .interior_ref_list_by_shop_id
        .delete_response(interior_ref_list.shop_id)
        .await;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_interior_ref_list_by_shop_id(
    shop_id: i32,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    env.caches
        .interior_ref_list_by_shop_id
        .get_response(shop_id, || async {
            let interior_ref_list = InteriorRefList::get_by_shop_id(&env.db, shop_id).await?;
            let reply = json(&interior_ref_list);
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await
}

// TODO: probably need a way to get by shop id instead
pub async fn get_merchandise_list(id: i32, env: Environment) -> Result<impl Reply, Rejection> {
    env.caches
        .merchandise_list
        .get_response(id, || async {
            let merchandise_list = MerchandiseList::get(&env.db, id).await?;
            let reply = json(&merchandise_list);
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await
}

pub async fn list_merchandise_lists(
    list_params: ListParams,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    env.caches
        .list_merchandise_lists
        .get_response(list_params.clone(), || async {
            let merchandise_lists = MerchandiseList::list(&env.db, &list_params).await?;
            let reply = json(&merchandise_lists);
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await
}

pub async fn create_merchandise_list(
    merchandise_list: MerchandiseList,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    let ref_list_with_owner_id = MerchandiseList {
        owner_id: Some(owner_id),
        ..merchandise_list
    };
    let saved_merchandise_list = ref_list_with_owner_id
        .create(&env.db)
        .await
        .map_err(reject_anyhow)?;
    let url = saved_merchandise_list
        .url(&env.api_url)
        .map_err(reject_anyhow)?;
    let reply = json(&saved_merchandise_list);
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    env.caches.list_merchandise_lists.clear().await;
    env.caches
        .merchandise_list_by_shop_id
        .delete_response(saved_merchandise_list.shop_id)
        .await;
    Ok(reply)
}

pub async fn update_merchandise_list(
    id: i32,
    merchandise_list: MerchandiseList,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    let merchandise_list_with_id_and_owner_id = if merchandise_list.owner_id.is_some() {
        MerchandiseList {
            id: Some(id),
            ..merchandise_list
        }
    } else {
        MerchandiseList {
            id: Some(id),
            owner_id: Some(owner_id),
            ..merchandise_list
        }
    };
    let updated_merchandise_list = merchandise_list_with_id_and_owner_id
        .update(&env.db, owner_id, id)
        .await
        .map_err(reject_anyhow)?;
    let url = updated_merchandise_list
        .url(&env.api_url)
        .map_err(reject_anyhow)?;
    let reply = json(&updated_merchandise_list);
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
    env.caches.merchandise_list.delete_response(id).await;
    env.caches
        .merchandise_list_by_shop_id
        .delete_response(updated_merchandise_list.shop_id)
        .await;
    env.caches.list_merchandise_lists.clear().await;
    Ok(reply)
}

pub async fn update_merchandise_list_by_shop_id(
    shop_id: i32,
    merchandise_list: MerchandiseList,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    let merchandise_list_with_owner_id = MerchandiseList {
        owner_id: Some(owner_id),
        ..merchandise_list
    };
    let updated_merchandise_list = merchandise_list_with_owner_id
        .update_by_shop_id(&env.db, owner_id, shop_id)
        .await
        .map_err(reject_anyhow)?;
    let url = updated_merchandise_list
        .url(&env.api_url)
        .map_err(reject_anyhow)?;
    let reply = json(&updated_merchandise_list);
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
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

pub async fn delete_merchandise_list(
    id: i32,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    let merchandise_list = MerchandiseList::get(&env.db, id)
        .await
        .map_err(reject_anyhow)?;
    MerchandiseList::delete(&env.db, owner_id, id)
        .await
        .map_err(reject_anyhow)?;
    env.caches.merchandise_list.delete_response(id).await;
    env.caches
        .merchandise_list_by_shop_id
        .delete_response(merchandise_list.shop_id)
        .await;
    env.caches.list_merchandise_lists.clear().await;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_merchandise_list_by_shop_id(
    shop_id: i32,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    env.caches
        .merchandise_list_by_shop_id
        .get_response(shop_id, || async {
            let merchandise_list = MerchandiseList::get_by_shop_id(&env.db, shop_id).await?;
            let reply = json(&merchandise_list);
            let reply = with_status(reply, StatusCode::OK);
            Ok(reply)
        })
        .await
}

pub async fn buy_merchandise(
    shop_id: i32,
    merchandise_params: MerchandiseParams,
    api_key: Option<Uuid>,
    env: Environment,
) -> Result<impl Reply, Rejection> {
    let owner_id = authenticate(&env, api_key).await.map_err(reject_anyhow)?;
    // TODO: create transaction
    let updated_merchandise_list = MerchandiseList::update_merchandise_quantity(
        &env.db,
        shop_id,
        &(merchandise_params.mod_name),
        merchandise_params.local_form_id,
        merchandise_params.quantity_delta,
    )
    .await
    .map_err(reject_anyhow)?;
    let url = updated_merchandise_list
        .url(&env.api_url)
        .map_err(reject_anyhow)?;
    let reply = json(&updated_merchandise_list);
    let reply = with_header(reply, "Location", url.as_str());
    let reply = with_status(reply, StatusCode::CREATED);
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
