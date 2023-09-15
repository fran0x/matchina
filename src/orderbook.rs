use std::{
    cmp::Reverse,
    collections::{btree_map::Entry, BTreeMap, VecDeque},
};

use indexmap::IndexMap;
use thiserror::Error;

use crate::order::{LimitOrder, OrderId, OrderPrice, OrderSide};

pub trait Handler {
    fn handle_create(&mut self, order: LimitOrder) -> Result<(), OrderbookError>;

    fn handle_cancel(&mut self, order_id: OrderId) -> Option<LimitOrder>;
}

const DEFAULT_LEVEL_SIZE: usize = 8;

#[derive(Default)]
pub struct Orderbook {
    _orders: IndexMap<OrderId, LimitOrder>,
    asks: BTreeMap<OrderPrice, VecDeque<OrderId>>,
    bids: BTreeMap<Reverse<OrderPrice>, VecDeque<OrderId>>,
}

impl Handler for Orderbook {
    fn handle_create(&mut self, mut incoming_order: LimitOrder) -> Result<(), OrderbookError> {
        let opposite = !incoming_order.side();
        while let (false, Some(top_order)) = (incoming_order.is_closed(), self.peek_mut(&opposite)) {
            let Some(_trade) = incoming_order.trade(top_order) else {
                break; // no match with top order, move on
            };

            if top_order.is_closed() {
                // if top order is completed remove from the book
                self.pop(&opposite).expect("no top order found");
            }
        }

        // insert incoming order in book if not completed
        if !incoming_order.is_closed() {
            self.insert(incoming_order);
        }

        Ok(())
    }

    #[inline]
    fn handle_cancel(&mut self, order_id: OrderId) -> Option<LimitOrder> {
        self.remove(&order_id)
    }
}

impl Orderbook {
    #[inline]
    fn insert(&mut self, order: LimitOrder) {
        match order.side() {
            OrderSide::Ask => self
                .asks
                .entry(order.limit_price())
                .or_insert_with(|| VecDeque::with_capacity(DEFAULT_LEVEL_SIZE)),
            OrderSide::Bid => self
                .bids
                .entry(Reverse(order.limit_price()))
                .or_insert_with(|| VecDeque::with_capacity(DEFAULT_LEVEL_SIZE)),
        }
        .push_back(order.id());

        self._orders.insert(order.id(), order);
    }

    #[inline]
    fn peek_mut(&mut self, side: &OrderSide) -> Option<&mut LimitOrder> {
        match side {
            OrderSide::Ask => self.asks.first_key_value().map(|(_, level)| level)?,
            OrderSide::Bid => self.bids.first_key_value().map(|(_, level)| level)?,
        }
        .front()
        .and_then(|order_id| self._orders.get_mut(order_id))
    }

    #[inline]
    fn pop(&mut self, side: &OrderSide) -> Option<LimitOrder> {
        match side {
            OrderSide::Ask => {
                let mut level = self.asks.first_entry()?;
                // prevents dangling levels
                if level.get().len() == 1 {
                    level.remove().pop_front()
                } else {
                    level.get_mut().pop_front()
                }
            }
            OrderSide::Bid => {
                let mut level = self.bids.first_entry()?;
                // prevents dangling levels
                if level.get().len() == 1 {
                    level.remove().pop_front()
                } else {
                    level.get_mut().pop_front()
                }
            }
        }
        .and_then(|order_id| self._orders.remove(&order_id))
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
