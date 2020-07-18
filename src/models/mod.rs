use serde::Deserialize;
use std::fmt;

pub mod model;
pub mod owner;
pub mod shop;

pub use model::Model;
pub use owner::Owner;
pub use shop::Shop;

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct ListParams {
    limit: Option<i64>,
    offset: Option<i64>,
    order_by: Option<String>,
    order: Option<Order>,
}

impl ListParams {
    pub fn get_order_by(&self) -> String {
        let default_order_by = "updated_at".to_string();
        let order_by = self.order_by.as_ref().unwrap_or(&default_order_by);
        let order = self.order.as_ref().unwrap_or(&Order::Desc);
        format!("{} {}", order_by, order)
    }
}
