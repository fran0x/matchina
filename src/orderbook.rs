use std::{
    cmp::Reverse,
    collections::{btree_map::Entry, BTreeMap, VecDeque},
    ops::{Deref, DerefMut},
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
    fn peek_top(&self, side: &OrderSide) -> Option<&Order>;

    fn handle_create(&mut self, order: Order) -> Result<(), OrderbookError>;

    fn handle_cancel(&mut self, order_id: OrderId) -> Option<Order>;
}

const DEFAULT_LEVEL_SIZE: usize = 8;

pub struct PriceLevel {
    order_ids: VecDeque<OrderId>,
    quantity: OrderQuantity,
}

impl Default for PriceLevel {
    fn default() -> Self {
        Self {
            order_ids: VecDeque::with_capacity(DEFAULT_LEVEL_SIZE),
            quantity: Decimal::ZERO,
        }
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
    pub fn can_trade(&self, order: &Order) -> bool {
        self.quantity.min(order.remaining()) != OrderQuantity::ZERO
    }

    #[inline]
    pub fn matches(&self, order: &Order) -> bool {
        let level = self;

        if level.is_closed() || order.is_closed() {
            return false;
        }

        let level_price = level.quantity;
        match order.limit_price() {
            // limit price == limit order
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

macro_rules! match_order {
    ($self:ident, $incoming_order:ident, $trades:ident, $order_ladder:ident, $opposite_ladder:ident) => {
        let mut drained_levels = 0;
        for (_, price_level) in $opposite_ladder.iter_mut() {
            if $incoming_order.is_closed() || !price_level.matches(&$incoming_order) {
                break;
            }

            let mut total_traded = OrderQuantity::ZERO;
            let mut total_trades = 0;

            for order_id in price_level.iter_mut() {
                let limit_order = $self.orders.get_mut(order_id).unwrap();
                let traded = $incoming_order.can_trade(limit_order);

                let trade = Trade::new(&mut $incoming_order, limit_order, traded).expect("there should be a trade");
                $trades.push(trade);

                total_traded += traded;
                total_trades += 1;
            }

            price_level.quantity -= total_traded;
            for _ in 0..total_trades {
                price_level.pop_front();
            }

            if price_level.quantity == OrderQuantity::ZERO {
                drained_levels += 1;
            }
        }
        for _ in 0..drained_levels {
            $opposite_ladder.pop_first();
        }
    };
}

#[derive(Default)]
pub struct Orderbook {
    orders: IndexMap<OrderId, Order>,
    asks: BTreeMap<OrderPrice, PriceLevel>,
    bids: BTreeMap<Reverse<OrderPrice>, PriceLevel>,
    trades: IndexMap<TradeId, Trade>,
}

impl Handler for Orderbook {
    #[inline]
    fn peek_top(&self, side: &OrderSide) -> Option<&Order> {
        match side {
            OrderSide::Ask => self.asks.first_key_value().map(|(_, level)| level)?,
            OrderSide::Bid => self.bids.first_key_value().map(|(_, level)| level)?,
        }
        .front()
        .and_then(|order_id| self.orders.get(order_id))
    }

    #[inline]
    fn handle_create(&mut self, mut incoming_order: Order) -> Result<(), OrderbookError> {
        let opposite = !incoming_order.side();

        let mut trades: Vec<Trade> = vec![];
        match opposite {
            OrderSide::Ask => {
                let order_ladder =  &mut self.bids;
                let opposite_ladder = &mut self.asks;
                match_order!(self, incoming_order, trades, order_ladder, opposite_ladder);
            }
            OrderSide::Bid => {
                let order_ladder = &mut self.asks;
                let opposite_ladder = &mut self.bids;
                match_order!(self, incoming_order, trades, order_ladder, opposite_ladder);
            }
        };

        for trade in trades {
            self.trades.insert(trade.id(), trade);
        }

        // insert incoming order if is bookable and is not completed
        if incoming_order.is_bookable() && !incoming_order.is_closed() {
            self.insert(incoming_order);
        }

        Ok(())
    }

    #[inline]
    fn handle_cancel(&mut self, order_id: OrderId) -> Option<Order> {
        self.remove(&order_id)
    }
}

impl Orderbook {
    #[inline]
    fn insert(&mut self, order: Order) {
        let limit_price = order
            .limit_price()
            .expect("only limit orders with limit price can be inserted");
        let price_level = match order.side() {
            OrderSide::Ask => self.asks.entry(limit_price).or_insert_with(|| PriceLevel::default()),
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
