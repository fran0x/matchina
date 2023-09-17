use std::fmt::Display;

use crate::{
    order::{OrderPrice, OrderSide},
    orderbook::Orderbook,
    orderbook::Watched,
};

pub struct Summary {
    pub orders: usize,
    pub trades: usize,
    pub best_bid: Option<OrderPrice>,
    pub best_ask: Option<OrderPrice>,
}

impl Summary {
    fn spread(&self) -> Option<OrderPrice> {
        match (self.best_bid, self.best_ask) {
            (Some(bid_price), Some(ask_price)) => Some(ask_price - bid_price),
            _ => None,
        }
    }
}

impl Display for Summary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Orders:{}, Trades: {}", self.orders, self.trades)?;
        writeln!(
            f,
            "BestBid:{:?}, BestAsk:{:?}, Spread: {:?}",
            self.best_bid,
            self.best_ask,
            self.spread()
        )
    }
}

pub fn compute(orderbook: &Orderbook) -> Summary {
    Summary {
        orders: orderbook.orders(),
        trades: orderbook.trades(),
        best_bid: orderbook.peek(&OrderSide::Bid).and_then(|bid| bid.limit_price()),
        best_ask: orderbook.peek(&OrderSide::Ask).and_then(|ask| ask.limit_price()),
    }
}
