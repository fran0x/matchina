use std::{
    cmp::Reverse,
    collections::{BTreeMap, VecDeque},
};

use indexmap::IndexMap;
use thiserror::Error;

use crate::order::{LimitOrder, Order, OrderId, OrderPrice, OrderSide};

pub trait Handler {
    fn handle_create(&mut self, _order: LimitOrder) -> Result<(), OrderbookError>;

    fn handle_cancel(&mut self, _order_id: OrderId) -> Option<Order>;
}

const _DEFAULT_LEVEL_SIZE: usize = 8;

#[derive(Default)]
pub struct Orderbook {
    _orders: IndexMap<OrderId, Order>,
    _asks: BTreeMap<OrderPrice, VecDeque<OrderId>>,
    _bids: BTreeMap<Reverse<OrderPrice>, VecDeque<OrderId>>,
}

impl Handler for Orderbook {
    fn handle_create(&mut self, _order: LimitOrder) -> Result<(), OrderbookError> {
        todo!()
    }

    #[inline]
    fn handle_cancel(&mut self, _order_id: OrderId) -> Option<Order> {
        todo!()
    }
}

impl Orderbook {
    #[inline]
    fn _insert(&mut self, order: LimitOrder) {
        match order.side() {
            OrderSide::Ask => self
                ._asks
                .entry(order.limit_price())
                .or_insert_with(|| VecDeque::with_capacity(_DEFAULT_LEVEL_SIZE)),
            OrderSide::Bid => self
                ._bids
                .entry(Reverse(order.limit_price()))
                .or_insert_with(|| VecDeque::with_capacity(_DEFAULT_LEVEL_SIZE)),
        }
        .push_back(order.id());

        self._orders.insert(order.id(), *order);
    }
}

#[derive(Debug, Error)]
pub enum OrderbookError {
    #[error("not cool")]
    PriceNotMatching,
}
