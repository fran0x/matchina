use std::fmt::Display;

use crate::{
    order::{OrderPrice, OrderSide},
    orderbook::Orderbook,
};

pub struct Summary {
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
        write!(f, "Best Bid:{:?} Best Ask:{:?} ", self.best_bid, self.best_ask)?;
        write!(f, "Spread: {:?}", self.spread())
    }
}

pub fn compute(orderbook: &Orderbook) -> Summary {
    Summary {
        best_bid: orderbook.peek_top(&OrderSide::Bid).and_then(|bid| bid.limit_price()),
        best_ask: orderbook.peek_top(&OrderSide::Ask).and_then(|ask| ask.limit_price()),
    }
}
