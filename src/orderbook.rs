use std::{
    cmp::Reverse,
    collections::{btree_map::Entry, BTreeMap, VecDeque},
};

use indexmap::IndexMap;
use thiserror::Error;

use crate::order::{LimitOrder, OrderId, OrderPrice, OrderSide};

pub trait Handler {
    fn handle_create(&mut self, _order: LimitOrder) -> Result<(), OrderbookError>;

    fn handle_cancel(&mut self, _order_id: OrderId) -> Option<LimitOrder>;
}

const _DEFAULT_LEVEL_SIZE: usize = 8;

#[derive(Default)]
pub struct Orderbook {
    _orders: IndexMap<OrderId, LimitOrder>,
    asks: BTreeMap<OrderPrice, VecDeque<OrderId>>,
    bids: BTreeMap<Reverse<OrderPrice>, VecDeque<OrderId>>,
}

impl Handler for Orderbook {
    fn handle_create(&mut self, _order: LimitOrder) -> Result<(), OrderbookError> {
        todo!()
    }

    #[inline]
    fn handle_cancel(&mut self, order_id: OrderId) -> Option<LimitOrder> {
        self.remove(&order_id)
    }
}

impl Orderbook {
    #[inline]
    fn _insert(&mut self, order: LimitOrder) {
        match order.side() {
            OrderSide::Ask => self
                .asks
                .entry(order.limit_price())
                .or_insert_with(|| VecDeque::with_capacity(_DEFAULT_LEVEL_SIZE)),
            OrderSide::Bid => self
                .bids
                .entry(Reverse(order.limit_price()))
                .or_insert_with(|| VecDeque::with_capacity(_DEFAULT_LEVEL_SIZE)),
        }
        .push_back(order.id());

        self._orders.insert(order.id(), order);
    }

    #[inline]
    fn remove(&mut self, order_id: &OrderId) -> Option<LimitOrder> {
        let order = self._orders.remove(order_id)?;
        let limit_price = order.limit_price();

        match order.side() {
            OrderSide::Ask => {
                let Entry::Occupied(mut level) = self.asks.entry(limit_price)
                else {
                    unreachable!();
                };

                // prevent dangling levels
                if level.get().len() == 1 {
                    level.remove().pop_front()
                } else {
                    level
                        .get()
                        .iter()
                        .position(|&order_id| order.id() == order_id)
                        .and_then(|index| level.get_mut().remove(index))
                }
            }
            OrderSide::Bid => {
                let Entry::Occupied(mut level) =
                    self.bids.entry(Reverse(limit_price))
                else {
                    unreachable!();
                };

                // prevent dangling levels
                if level.get().len() == 1 {
                    level.remove().pop_front()
                } else {
                    level
                        .get()
                        .iter()
                        .position(|&order_id| order.id() == order_id)
                        .and_then(|index| level.get_mut().remove(index))
                }
            }
        };

        Some(order)
    }
}

#[derive(Debug, Error)]
pub enum OrderbookError {
    #[error("not cool")]
    PriceNotMatching,
}
