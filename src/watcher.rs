use crate::{
    order::{OrderPrice, OrderSide},
    orderbook::Orderbook,
    orderbook::Watched,
};

struct _TopOfBook {
    ask: Option<OrderPrice>,
    bid: Option<OrderPrice>,
}

impl _TopOfBook {
    fn _spread(&self) -> Option<OrderPrice> {
        match (self.ask, self.bid) {
            (Some(ask_price), Some(bid_price)) => Some(ask_price - bid_price),
            _ => None,
        }
    }
}

fn _top_of_book(orderbook: &Orderbook) -> _TopOfBook {
    _TopOfBook {
        ask: orderbook.peek(&OrderSide::Ask).and_then(|ask| ask.limit_price()),
        bid: orderbook.peek(&OrderSide::Bid).and_then(|bid| bid.limit_price()),
    }
}
