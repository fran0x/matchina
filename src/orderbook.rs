use std::{
    cmp::Reverse,
    collections::{btree_map::Entry, BTreeMap, VecDeque}, ops::{Deref, DerefMut},
};

use indexmap::IndexMap;
use num::Zero;
use rust_decimal::Decimal;
use thiserror::Error;

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
}

const DEFAULT_LEVEL_SIZE: usize = 8;

pub struct PriceLevel {
    order_ids: VecDeque<OrderId>,
    quantity: OrderQuantity,
}

impl Default for PriceLevel {
    fn default() -> Self {
        Self { order_ids: VecDeque::with_capacity(DEFAULT_LEVEL_SIZE), quantity: Decimal::ZERO }
    }
}

impl Deref for PriceLevel {
    type Target = VecDeque<OrderId>;

    fn deref(&self) -> &Self::Target {
        &self.order_ids
    }
}

impl DerefMut for PriceLevel {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.order_ids
    }
}

impl PriceLevel {
    #[inline]
    pub fn matches(&self, order: &Order) -> bool {
        let level = self;

        if level.is_closed() || order.is_closed() {
            return false;
        }

        let level_price = level.quantity;
        match order.limit_price() { // limit price == limit order
            Some(limit_price) => match order.side() {
                OrderSide::Ask => limit_price <= level_price,
                OrderSide::Bid => limit_price >= level_price,
            },
            None => true, // no limit price == market order
        }
    }

    fn is_closed(&self) -> bool {
        self.quantity != OrderQuantity::zero()
    }
}
#[derive(Default)]
pub struct Orderbook {
    orders: IndexMap<OrderId, Order>,
    asks: BTreeMap<OrderPrice, PriceLevel>,
    bids: BTreeMap<Reverse<OrderPrice>, PriceLevel>,
    trades: IndexMap<TradeId, Trade>,
}

impl Handler for Orderbook {
    fn handle_create(&mut self, mut order: Order) -> Result<(), OrderbookError> {
        let opposite = !order.side();

        let mut trades = vec![];
        while let (false, Some(price_level)) = (order.is_closed(), self.peek_(&opposite)) {
            for order_id in & price_level.order_ids {
                let maker = self.orders.get(order_id).unwrap();
                let traded = order.can_trade(maker);
                let trade = Trade::new(&mut order, maker, traded).expect("there should be a trade");
                trades.push(trade);
                
                price_level.quantity -= traded;
                if maker.is_closed() {
                    price_level.pop_front().expect("msg");
                }
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
            OrderSide::Ask => self.asks.iter_mut().next().map(|(_, level)| level)?,
            OrderSide::Bid => self.bids.iter_mut().next().map(|(_, level)| level)?,
        }
        .front()
        .and_then(|order_id| self.orders.get_mut(order_id))
    }
}

impl Orderbook {
    #[inline]
    fn peek_(&mut self, side: &OrderSide) -> Option<&mut PriceLevel> {
        match side {
            OrderSide::Ask => self.asks.iter_mut().next().map(|(_, level)| level),
            OrderSide::Bid => self.bids.iter_mut().next().map(|(_, level)| level),
        }
    }

    #[inline]
    fn insert(&mut self, order: Order) {
        let limit_price = order
            .limit_price()
            .expect("only limit orders with limit price can be inserted");
        let price_level = match order.side() {
            OrderSide::Ask => self
                .asks
                .entry(limit_price)
                .or_insert_with(|| PriceLevel::default()),
            OrderSide::Bid => self
                .bids
                .entry(Reverse(limit_price))
                .or_insert_with(|| PriceLevel::default()),
        };

        price_level.quantity += order.remaining();
        price_level.push_back(order.id());

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
                let Entry::Occupied(mut price_level) = self.asks.entry(limit_price) else {
                    unreachable!();
                };

                // prevent dangling levels
                if price_level.get().len() == 1 {
                    price_level.remove().pop_front()
                } else {
                    price_level.get_mut().quantity -= order.remaining();
                    price_level
                        .get()
                        .iter()
                        .position(|&order_id| order.id() == order_id)
                        .and_then(|idx| price_level.get_mut().remove(idx))
                }
            }
            OrderSide::Bid => {
                let Entry::Occupied(mut price_level) = self.bids.entry(Reverse(limit_price)) else {
                    unreachable!();
                };

                // prevent dangling levels
                if price_level.get().len() == 1 {
                    price_level.remove().pop_front()
                } else {
                    price_level.get_mut().quantity -= order.remaining();
                    price_level
                        .get()
                        .iter()
                        .position(|&order_id| order.id() == order_id)
                        .and_then(|idx| price_level.get_mut().remove(idx))
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
