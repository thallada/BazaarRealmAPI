use serde::Deserialize;
use std::fmt;
use std::hash::Hash;

pub mod interior_ref_list;
pub mod merchandise_list;
pub mod model;
pub mod owner;
pub mod shop;
pub mod transaction;

pub use interior_ref_list::InteriorRefList;
pub use merchandise_list::{MerchandiseList, MerchandiseParams};
pub use model::{Model, UpdateableModel};
pub use owner::Owner;
pub use shop::Shop;
pub use transaction::Transaction;

#[derive(Debug, Eq, PartialEq, Hash, Clone, Deserialize)]
pub enum Order {
    Asc,
    Desc,
}

impl fmt::Display for Order {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Order::Asc => "ASC",
                Order::Desc => "DESC",
            }
        )
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, Deserialize)]
pub struct ListParams {
    limit: Option<i64>,
    offset: Option<i64>,
    order_by: Option<String>,
    order: Option<Order>,
}

impl ListParams {
    pub fn get_order_by(&self) -> Option<String> {
        if let Some(order_by) = self.order_by.as_ref() {
            let order = self.order.as_ref().unwrap_or(&Order::Desc);
            return Some(format!("{} {}", order_by, order));
        }
        None
    }
}
