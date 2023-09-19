use std::{
    cmp::Reverse,
    collections::{btree_map::Entry, BTreeMap, VecDeque},
};

use indexmap::IndexMap;
use num::Zero;
use thiserror::Error;
use tracing::info;

use crate::{
    order::{Order, OrderId, OrderPrice, OrderQuantity, OrderSide},
    trade::{Trade, TradeId},
};

pub trait Handler {
    fn handle_create(&mut self, order: Order) -> Result<(), OrderbookError>;

    fn handle_cancel(&mut self, order_id: OrderId) -> Option<Order>;
}

pub trait Scanner {
    fn peek(&self, side: &OrderSide) -> Option<&Order>;

    fn peek_mut(&mut self, side: &OrderSide) -> Option<&mut Order>;

    fn matches(&mut self, order: &Order) -> (OrderQuantity, Vec<&mut Order>);
}

const DEFAULT_LEVEL_SIZE: usize = 8;

#[derive(Default)]
pub struct Orderbook {
    orders: IndexMap<OrderId, Order>,
    asks: BTreeMap<OrderPrice, VecDeque<OrderId>>,
    bids: BTreeMap<Reverse<OrderPrice>, VecDeque<OrderId>>,
    trades: IndexMap<TradeId, Trade>,
}

impl Handler for Orderbook {
    fn handle_create(&mut self, mut order: Order) -> Result<(), OrderbookError> {
        let opposite = !order.side();

        let mut trades = vec![];
        while let (false, Some(top_order)) = (order.is_closed(), self.peek_mut(&opposite)) {
            if let Some(trade) = order.trade(top_order) {
                trades.push(trade);
            } else {
                break; // no match with top order, move on
            };

            if top_order.is_closed() {
                // if top order is completed remove from the book
                self.pop(&opposite).expect("no top order found");
            }
        }

        for trade in trades {
            //info!("{trade}");
            self.trades.insert(trade.id(), trade);
        }

        // insert incoming order if is bookable and is not completed
        if order.is_bookable() && !order.is_closed() {
            self.insert(order);
        }

        Ok(())
    }

    #[inline]
    fn handle_cancel(&mut self, order_id: OrderId) -> Option<Order> {
        self.remove(&order_id)
    }
}

impl Scanner for Orderbook {
    #[inline]
    fn peek(&self, side: &OrderSide) -> Option<&Order> {
        match side {
            OrderSide::Ask => self.asks.first_key_value().map(|(_, level)| level)?,
            OrderSide::Bid => self.bids.first_key_value().map(|(_, level)| level)?,
        }
        .front()
        .and_then(|order_id| self.orders.get(order_id))
    }

    #[inline]
    fn peek_mut(&mut self, side: &OrderSide) -> Option<&mut Order> {
        match side {
            OrderSide::Ask => self.asks.first_key_value().map(|(_, level)| level)?,
            OrderSide::Bid => self.bids.first_key_value().map(|(_, level)| level)?,
        }
        .front()
        .and_then(|order_id| self.orders.get_mut(order_id))
    }

    fn matches(&mut self, taker: &Order) -> (OrderQuantity, Vec<&mut Order>) {
        let mut matched_orders = vec!();
        let mut remaining = taker.remaining();

        for (_, order_ids) in &self.asks {
            for order_id in order_ids {
                if let Some(order) = self.orders.get_mut(order_id) {
                    if taker.matches(order) {
                        let traded = order.remaining().min(remaining);
                        remaining -= traded;
                        matched_orders.push(order);
                    }
                }
            }
            if remaining == OrderQuantity::zero() {
                break;
            }
        }

        (remaining, matched_orders)
    }
}

impl Orderbook {
    #[inline]
    fn insert(&mut self, order: Order) {
        let limit_price = order
            .limit_price()
            .expect("only limit orders with limit price can be inserted");
        match order.side() {
            OrderSide::Ask => self
                .asks
                .entry(limit_price)
                .or_insert_with(|| VecDeque::with_capacity(DEFAULT_LEVEL_SIZE)),
            OrderSide::Bid => self
                .bids
                .entry(Reverse(limit_price))
                .or_insert_with(|| VecDeque::with_capacity(DEFAULT_LEVEL_SIZE)),
        }
        .push_back(order.id());

        self.orders.insert(order.id(), order);
    }

    #[inline]
    fn remove(&mut self, order_id: &OrderId) -> Option<Order> {
        let order = self.orders.remove(order_id)?;
        let limit_price = order
            .limit_price()
            .expect("only limit orders with limit price can be removed");

        match order.side() {
            OrderSide::Ask => {
                let Entry::Occupied(mut level) = self.asks.entry(limit_price) else {
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
                let Entry::Occupied(mut level) = self.bids.entry(Reverse(limit_price)) else {
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

    #[inline]
    fn pop(&mut self, side: &OrderSide) -> Option<Order> {
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
        .and_then(|order_id| self.orders.remove(&order_id))
    }
}

#[derive(Debug, Error)]
pub enum OrderbookError {
    #[error("not cool")]
    PriceNotMatching,
}
