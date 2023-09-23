use std::{
    cmp::Reverse,
    collections::{btree_map::Entry, BTreeMap, VecDeque},
    fmt::Display,
    ops::{Deref, DerefMut},
};

use indexmap::IndexMap;
use num::Zero;
use rust_decimal::Decimal;
use thiserror::Error;
use anyhow::Result;

use crate::{
    order::{Order, OrderId, OrderPrice, OrderQuantity, OrderSide},
    trade::{Trade, TradeId, TradeError},
};

const DEFAULT_LEVEL_SIZE: usize = 8;

trait Ladder: Deref + DerefMut {
    fn insert(&mut self, order: &Order) -> Result<&mut Self, OrderbookError>;

    fn remove(&mut self, order: &Order) -> Result<&mut Self, OrderbookError>;
}

#[derive(Default)]
struct LadderWrapper<T>(T);

impl<T> Deref for LadderWrapper<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for LadderWrapper<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Ladder for LadderWrapper<BTreeMap<Reverse<OrderPrice>, PriceLevel>> {
    fn insert(&mut self, order: &Order) -> Result<&mut Self, OrderbookError> {
        let limit_price = order.limit_price().ok_or(OrderbookError::OrderToInsertWithNoLimitPrice(*order))?;
        let price_level = self
            .0
            .entry(Reverse(limit_price))
            .or_insert_with(|| PriceLevel::default());

        price_level.quantity += order.remaining();
        price_level.push_back(order.id());

        Ok(self)
    }

    fn remove(&mut self, order: &Order) -> Result<&mut Self, OrderbookError> {
        let limit_price = order.limit_price().ok_or(OrderbookError::OrderToRemoveWithNoLimitPrice(*order))?;
        let Entry::Occupied(mut price_level) = self.0.entry(Reverse(limit_price)) else {
            unreachable!();
        };

        if price_level.get().len() == 1 {
            price_level.remove();
        } else {
            let price_level = price_level.get_mut();
            price_level.quantity -= order.remaining();
            if let Some(idx) = price_level.iter().position(|&order_id| order.id() == order_id) {
                price_level.remove(idx);
            }
        }

        Ok(self)
    }
}

impl Ladder for LadderWrapper<BTreeMap<OrderPrice, PriceLevel>> {
    fn insert(&mut self, order: &Order) -> Result<&mut Self, OrderbookError> {
        let limit_price = order.limit_price().ok_or(OrderbookError::OrderToInsertWithNoLimitPrice(*order))?;
        let price_level = self.0.entry(limit_price).or_insert_with(|| PriceLevel::default());

        price_level.quantity += order.remaining();
        price_level.push_back(order.id());

        Ok(self)
    }

    fn remove(&mut self, order: &Order) -> Result<&mut Self, OrderbookError> {
        let limit_price = order.limit_price().ok_or(OrderbookError::OrderToRemoveWithNoLimitPrice(*order))?;
        let Entry::Occupied(mut price_level) = self.0.entry(limit_price) else {
            unreachable!();
        };

        if price_level.get().len() == 1 {
            price_level.remove();
        } else {
            let price_level = price_level.get_mut();
            price_level.quantity -= order.remaining();
            if let Some(idx) = price_level.iter().position(|&order_id| order.id() == order_id) {
                price_level.remove(idx);
            }
        }

        Ok(self)
    }
}

type AsksLadder = LadderWrapper<BTreeMap<OrderPrice, PriceLevel>>;
type BidsLadder = LadderWrapper<BTreeMap<Reverse<OrderPrice>, PriceLevel>>;

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
    #[inline]
    fn is_closed(&self) -> bool {
        self.quantity != OrderQuantity::zero()
    }

    #[inline]
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
}

impl Display for PriceLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} [{:?}]", self.quantity, self.order_ids)
    }
}

macro_rules! match_order {
    ($incoming_order:ident, $orders:ident, $trades:ident, $order_ladder:ident, $opposite_ladder:ident) => {
        let mut trades: Vec<Trade> = vec![];
        let mut drained_levels = 0;

        for (_, price_level) in $opposite_ladder.iter_mut() {
            if $incoming_order.is_closed() || !price_level.matches(&$incoming_order) {
                break;
            }

            let mut total_traded = OrderQuantity::ZERO;
            let mut total_trades = 0;

            for order_id in price_level.iter_mut() {
                let limit_order = $orders
                    .get_mut(order_id)
                    .expect("a limit order is expected in the price level");
                let traded = $incoming_order.can_trade(limit_order);

                let trade = Trade::new(&mut $incoming_order, limit_order, traded).expect("there should be a trade");
                trades.push(trade);

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

        // record trades
        for trade in trades {
            $trades.insert(trade.id(), trade);
        }

        // insert limit order in the book
        if !$incoming_order.is_closed() && $incoming_order.is_bookable() {
            $order_ladder.insert(&$incoming_order)?;
            $orders.insert($incoming_order.id(), $incoming_order);
        }
    };
}

#[derive(Default)]
pub struct Orderbook {
    asks: AsksLadder,
    bids: BidsLadder,
    orders: IndexMap<OrderId, Order>,
    trades: IndexMap<TradeId, Trade>,
}

impl Orderbook {
    #[inline]
    pub fn peek_top(&self, side: &OrderSide) -> Option<&Order> {
        match side {
            OrderSide::Ask => self.asks.first_key_value().map(|(_, level)| level)?,
            OrderSide::Bid => self.bids.first_key_value().map(|(_, level)| level)?,
        }
        .front()
        .and_then(|order_id| self.orders.get(order_id))
    }

    #[inline]
    pub fn handle_create(&mut self, mut order: Order) -> Result<(), OrderbookError> {
        let orders = &mut self.orders;
        let trades = &mut self.trades;

        match order.side() {
            OrderSide::Ask => {
                let order_ladder = &mut self.asks;
                let opposite_ladder = &mut self.bids;

                let mut new_trades: Vec<Trade> = vec![];
                let mut drained_levels = 0;

                for (_, price_level) in opposite_ladder.iter_mut() {
                    if order.is_closed() || !price_level.matches(&order) {
                        break;
                    }

                    let mut total_traded = OrderQuantity::ZERO;
                    let mut total_trades = 0;

                    for order_id in price_level.iter_mut() {
                        let limit_order = orders
                            .get_mut(order_id).ok_or(OrderbookError::OrderToMatchNotFound(*order_id))?;
                        let traded = order.can_trade(limit_order);

                        let trade = Trade::new(&mut order, limit_order, traded).map_err(OrderbookError::TradeError)?;
                        new_trades.push(trade);

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
                    opposite_ladder.pop_first();
                }

                // record trades
                for trade in new_trades {
                    trades.insert(trade.id(), trade);
                }

                // insert limit order in the book
                if !order.is_closed() && order.is_bookable() {
                    order_ladder.insert(&order)?;
                    orders.insert(order.id(), order);
                }
            }
            OrderSide::Bid => {
                let order_ladder = &mut self.bids;
                let opposite_ladder = &mut self.asks;
                match_order!(order, orders, trades, order_ladder, opposite_ladder);
            }
        };

        Ok(())
    }

    #[inline]
    pub fn handle_cancel(&mut self, order_id: OrderId) -> Result<Order, OrderbookError> {
        let order = self
            .orders
            .remove(&order_id)
            .ok_or(OrderbookError::OrderToCancelNotFound(order_id))?;

        match order.side() {
            OrderSide::Ask => {
                let order_ladder = &mut self.asks;
                order_ladder.remove(&order)?;
            }
            OrderSide::Bid => {
                let order_ladder = &mut self.bids;
                order_ladder.remove(&order)?;
            }
        }

        Ok(order)
    }
}

#[derive(Debug, Error)]
pub enum OrderbookError {
    #[error("order cannot be inserted into the book with no limit price! {0}")]
    OrderToInsertWithNoLimitPrice(Order),
    #[error("order cannot be removed from the book with no limit price! {0}")]
    OrderToRemoveWithNoLimitPrice(Order),
    #[error("order to cancel not found in the book! {0}")]
    OrderToCancelNotFound(OrderId),
    #[error("order to match not found in the book! {0}")]
    OrderToMatchNotFound(OrderId),
    #[error("trade error: {0}")]
    TradeError(#[from] TradeError),
}

#[cfg(test)]
mod test {
    use rstest::{fixture, rstest};

    use super::*;
    use crate::order::{Order, OrderSide};

    // convention for order ids: side (bid = 0, ask = 1), 3-digit quantity, 3-digit price (for market orders always 999)

    #[fixture]
    fn orderbook() -> Orderbook {
        Orderbook::default()
    }

    #[fixture]
    fn ask_100_at_015() -> Order {
        let order_id = OrderId::new(1_100_015);
        Order::limit_order(order_id, OrderSide::Ask, 100.into(), 015.into())
    }

    #[fixture]
    fn ask_080_at_015() -> Order {
        let order_id = OrderId::new(1_080_015);
        Order::limit_order(order_id, OrderSide::Ask, 080.into(), 015.into())
    }

    #[fixture]
    fn ask_070_at_014() -> Order {
        let order_id = OrderId::new(1_070_014);
        Order::limit_order(order_id, OrderSide::Ask, 070.into(), 014.into())
    }

    #[fixture]
    fn bid_025_at_014() -> Order {
        let order_id = OrderId::new(0_025_014);
        Order::limit_order(order_id, OrderSide::Bid, 025.into(), 014.into())
    }

    #[fixture]
    fn bid_099_at_015() -> Order {
        let order_id = OrderId::new(0_099_015);
        Order::limit_order(order_id, OrderSide::Bid, 099.into(), 015.into())
    }

    #[fixture]
    fn bid_020_at_016() -> Order {
        let order_id = OrderId::new(0_020_016);
        Order::limit_order(order_id, OrderSide::Bid, 020.into(), 016.into())
    }

    mod limit_orders_no_match {
        use super::*;

        #[rstest]
        fn test_handle_create(mut orderbook: Orderbook, ask_100_at_015: Order, bid_025_at_014: Order) {
            // different side not matching
            assert_ne!(ask_100_at_015.side(), bid_025_at_014.side());
            assert!(!bid_025_at_014.matches(&ask_100_at_015));
    
            assert!(orderbook.handle_create(ask_100_at_015).is_ok());
            assert!(orderbook.handle_create(bid_025_at_014).is_ok());
    
            // confirm the top for bid and the top for ask are the ones inserted
            assert_eq!(orderbook.peek_top(&OrderSide::Ask), Some(&ask_100_at_015));
            assert_eq!(orderbook.peek_top(&OrderSide::Bid), Some(&bid_025_at_014));
        }
    
        #[rstest]
        fn test_handle_cancel(mut orderbook: Orderbook, ask_100_at_015: Order, bid_025_at_014: Order) {
            // different side not matching
            assert_ne!(ask_100_at_015.side(), bid_025_at_014.side());
            assert!(!bid_025_at_014.matches(&ask_100_at_015));
    
            let _ = orderbook.handle_create(ask_100_at_015);
            let _ = orderbook.handle_create(bid_025_at_014);
    
            // cancel the ask then confirm the top ask is empty and the top bid remains, finally try to cancel the same again
            assert_eq!(orderbook.handle_cancel(ask_100_at_015.id()).ok(), Some(ask_100_at_015));
            assert_eq!(orderbook.peek_top(&OrderSide::Ask), None);
            assert_eq!(orderbook.peek_top(&OrderSide::Bid), Some(&bid_025_at_014));
            assert_eq!(orderbook.handle_cancel(ask_100_at_015.id()).ok(), None);
        }
    
        #[rstest]
        fn test_handle_create_same_level(mut orderbook: Orderbook, ask_100_at_015: Order, ask_080_at_015: Order) {
            // same side same price
            assert_eq!(ask_100_at_015.side(), ask_080_at_015.side());
            assert_eq!(ask_100_at_015.limit_price(), ask_080_at_015.limit_price());
    
            let _ = orderbook.handle_create(ask_100_at_015);
            let _ = orderbook.handle_create(ask_080_at_015);
    
            // confirm the first ask is the one returned as top then cancel that one and confirm the other becomes the new top
            assert_eq!(orderbook.peek_top(&OrderSide::Ask), Some(&ask_100_at_015));
            assert_eq!(orderbook.handle_cancel(ask_100_at_015.id()).ok(), Some(ask_100_at_015));
            assert_eq!(orderbook.peek_top(&OrderSide::Ask), Some(&ask_080_at_015));
        }
    
        #[rstest]
        fn test_handle_create_different_ask_price(mut orderbook: Orderbook, ask_100_at_015: Order, ask_070_at_014: Order) {
            // same side different price (second order is a better ask)
            assert_eq!(ask_100_at_015.side(), ask_070_at_014.side());
            assert!(
                ask_100_at_015.limit_price().unwrap() >
                ask_070_at_014.limit_price().unwrap()
            );

            // after next 2 lines we should have 2 ask levels with the second at the top
            let _ = orderbook.handle_create(ask_100_at_015);
            let _ = orderbook.handle_create(ask_070_at_014);
    
            // confirm the second ask is the one returned as top then cancel that one and confirm the other becomes the new top
            assert_eq!(orderbook.peek_top(&OrderSide::Ask), Some(&ask_070_at_014));
            assert_eq!(orderbook.handle_cancel(ask_070_at_014.id()).ok(), Some(ask_070_at_014));
            assert_eq!(orderbook.peek_top(&OrderSide::Ask), Some(&ask_100_at_015));
        }
    
        #[rstest]
        fn test_handle_create_different_bid_price(mut orderbook: Orderbook, bid_099_at_015: Order, bid_020_at_016: Order) {
            // after next 2 lines we should have 2 bid levels with the second at the top
            let _ = orderbook.handle_create(bid_099_at_015);
            let _ = orderbook.handle_create(bid_020_at_016);
    
            // confirm the second bid is the one returned as top then cancel that one and confirm the other becomes the new top
            assert_eq!(orderbook.peek_top(&OrderSide::Bid), Some(&bid_020_at_016));
            assert_eq!(orderbook.handle_cancel(bid_020_at_016.id()).ok(), Some(bid_020_at_016));
            assert_eq!(orderbook.peek_top(&OrderSide::Bid), Some(&bid_099_at_015));
        }
    }

    mod limit_orders_match {
        use super::*;

        #[rstest]
        fn test_handle_create_match_at_level(mut orderbook: Orderbook, ask_100_at_015: Order, bid_099_at_015: Order) {
            // different side AND matching
            assert_ne!(ask_100_at_015.side(), bid_099_at_015.side());
            assert!(bid_099_at_015.matches(&ask_100_at_015));
    
            assert!(orderbook.handle_create(bid_099_at_015).is_ok());
    
            // the ask is completed and no bid is left
            assert_eq!(orderbook.peek_top(&OrderSide::Ask), Some(&ask_100_at_015));
            assert_eq!(orderbook.peek_top(&OrderSide::Bid), None);
        }
    }
}
