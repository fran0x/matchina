use std::{
    cmp::Reverse,
    collections::{BTreeMap, VecDeque},
    marker::PhantomData,
};

use indexmap::IndexMap;
use thiserror::Error;

use crate::{
    order::{Order, OrderId, OrderPrice},
    trade::Trade,
};

pub struct Orderbook {
    _orders: IndexMap<OrderId, Order>,
    _asks: BTreeMap<OrderPrice, VecDeque<OrderId>>,
    _bids: BTreeMap<Reverse<OrderPrice>, VecDeque<OrderId>>,
    _trade: PhantomData<Trade>,
}

impl Default for Orderbook {
    fn default() -> Self {
        Self {
            _orders: IndexMap::default(),
            _asks: BTreeMap::default(),
            _bids: BTreeMap::default(),
            _trade: PhantomData,
        }
    }
}
impl Orderbook {
    pub fn r#match(&mut self, _order: Order) -> Result<(), OrderbookError> {
        todo!()
    }

    #[inline]
    pub fn remove(&mut self, _order_id: OrderId) -> Option<Order> {
        todo!()
    }
}

#[derive(Debug, Error)]
pub enum OrderbookError {
    #[error("not cool")]
    PriceNotMatching,
}
