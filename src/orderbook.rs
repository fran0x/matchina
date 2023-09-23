use std::{
    cmp::Reverse,
    collections::{btree_map::Entry, BTreeMap, VecDeque},
    ops::{Deref, DerefMut}, fmt::Display,
};

use indexmap::IndexMap;
use num::Zero;
use rust_decimal::Decimal;
use thiserror::Error;

use crate::{
    order::{Order, OrderId, OrderPrice, OrderQuantity, OrderSide},
    trade::{Trade, TradeId},
};

const DEFAULT_LEVEL_SIZE: usize = 8;

trait Ladder: Deref + DerefMut {
    fn insert(&mut self, order: &Order) -> &mut Self;

    fn remove(&mut self, order: &Order) -> &mut Self;
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
    fn insert(&mut self, order: &Order) -> &mut Self {
        let limit_price = order.limit_price().expect("");
        let price_level = self.0.entry(Reverse(limit_price)).or_insert_with(|| PriceLevel::default());
        
        price_level.quantity += order.remaining();
        price_level.push_back(order.id());

        self
    }

    fn remove(&mut self, order: &Order) -> &mut Self {
        let limit_price = order.limit_price().expect("");
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

        self
    }
}

impl Ladder for LadderWrapper<BTreeMap<OrderPrice, PriceLevel>> {
    fn insert(&mut self, order: &Order) -> &mut Self {
        let limit_price = order.limit_price().expect("a limit price is required to insert in the ladder");
        let price_level = self.0.entry(limit_price).or_insert_with(|| PriceLevel::default());
        
        price_level.quantity += order.remaining();
        price_level.push_back(order.id());
        price_level.push_back(order.id());

        self
    }

    fn remove(&mut self, order: &Order) -> &mut Self {
        let limit_price = order.limit_price().expect("a limit price is required to remove in the ladder");
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

        self
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
                let limit_order = $orders.get_mut(order_id).expect("a limit order is expected in the price level");
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
            $order_ladder.insert(& $incoming_order);
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
                        let limit_order = orders.get_mut(order_id).expect("a limit order is expected in the price level");
                        let traded = order.can_trade(limit_order);
        
                        let trade = Trade::new(&mut order, limit_order, traded).expect("there should be a trade");
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
                    order_ladder.insert(&order);
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
    pub fn handle_cancel(&mut self, order_id: OrderId) -> Option<Order> {
        let order = self.orders.remove(&order_id).expect("a limit order to cancel should be found");

        match order.side() {
            OrderSide::Ask => {
                let order_ladder = &mut self.asks;
                order_ladder.remove(& order);
            }
            OrderSide::Bid => {
                let order_ladder = &mut self.bids;
                order_ladder.remove(& order);
            }
        }

        Some(order)
    }
}

#[derive(Debug, Error)]
pub enum OrderbookError {
    #[error("not cool")]
    PriceNotMatching,
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::order::{Order, OrderSide};

    #[test]
    fn test_handle_create() {
        let mut orderbook = Orderbook::default();

        let ask_id: OrderId = 1.into();
        let ask = Order::limit_order(ask_id, OrderSide::Ask, 100.into(), 15.into());

        let result = orderbook.handle_create(ask);

        assert!(result.is_ok());
        assert_eq!(orderbook.peek_top(&OrderSide::Ask), Some(&ask));
    }
}