use std::{
    fmt::Display,
    sync::atomic::{AtomicU64, Ordering::Relaxed},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::order::{Order, OrderError, OrderId, OrderPrice, OrderQuantity};

#[derive(Clone, Copy, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct TradeId(u64);

impl TradeId {
    #[inline]
    pub fn new(trade_id: u64) -> Self {
        Self(trade_id)
    }
}

impl From<u64> for TradeId {
    fn from(value: u64) -> TradeId {
        TradeId::new(value)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Trade {
    id: TradeId,
    taker: OrderId,
    maker: OrderId,
    price: OrderPrice,
    quantity: OrderQuantity,
}

impl Trade {
    #[inline]
    pub fn new(taker: &mut Order, maker: &mut Order, traded: OrderQuantity) -> Result<Trade, TradeError> {
        let price = maker
            .limit_price()
            .ok_or(TradeError::MakerWithoutLimitPrice(maker.id()))?;

        taker.fill(traded).map_err(TradeError::OrderError)?;
        maker.fill(traded).map_err(TradeError::OrderError)?;

        static TRADE_ID_GENERATOR: AtomicU64 = AtomicU64::new(0);
        let trade_id = TRADE_ID_GENERATOR.fetch_add(1, Relaxed);

        Ok(Trade {
            id: trade_id.into(),
            taker: taker.id(),
            maker: maker.id(),
            price,
            quantity: traded,
        })
    }

    #[inline]
    pub fn id(&self) -> TradeId {
        self.id
    }

    #[inline]
    pub fn price(&self) -> OrderPrice {
        self.price
    }
}

impl Display for Trade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TRADE[{:?}] [taker:{:?}|maker:{:?}] {}@{}",
            self.id, self.taker, self.maker, self.quantity, self.price
        )
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum TradeError {
    #[error("maker should be a limit order, always with a limit price! {0}")]
    MakerWithoutLimitPrice(OrderId),
    #[error("order error: {0}")]
    OrderError(#[from] OrderError),
}

#[cfg(test)]
mod test {
    use crate::{
        order::{Order, OrderId, OrderQuantity, OrderSide},
        trade::Trade,
    };

    use rstest::{fixture, rstest};

    // convention for order ids: 3-digit side (bid = 900, ask = 901), 3-digit quantity, 3-digit price (for market orders always 999)

    #[fixture]
    fn ask_010_at_100() -> Order {
        let order_id = OrderId::new(901_010_100);
        Order::limit_order(order_id, OrderSide::Ask, 10.into(), 100.into())
    }

    #[fixture]
    fn bid_015_at_100() -> Order {
        let order_id = OrderId::new(900_015_100);
        Order::limit_order(order_id, OrderSide::Bid, 15.into(), 100.into())
    }

    #[fixture]
    fn bid_015_at_market() -> Order {
        let order_id = OrderId::new(900_015_999);
        Order::market_order(order_id, OrderSide::Bid, 15.into())
    }

    #[rstest]
    fn match_limit_order(bid_015_at_100: Order, ask_010_at_100: Order) {
        let mut taker = bid_015_at_100;
        let mut maker = ask_010_at_100;

        let traded = taker.remaining().min(maker.remaining());
        assert_eq!(traded, taker.can_trade(&maker));

        let trade = Trade::new(&mut taker, &mut maker, traded);
        assert!(trade.is_ok());

        // check that the orders have been filled correctly
        assert_eq!(taker.remaining(), 5.into());
        assert_eq!(maker.remaining(), OrderQuantity::ZERO);

        // check the details of the trade
        let trade = trade.unwrap();
        assert_eq!(trade.taker, taker.id());
        assert_eq!(trade.maker, maker.id());
        assert_eq!(trade.quantity, traded);
        assert_eq!(trade.price, maker.limit_price().unwrap());
    }

    #[rstest]
    fn match_market_order(bid_015_at_market: Order, ask_010_at_100: Order) {
        let mut taker = bid_015_at_market;
        let mut maker = ask_010_at_100;

        let traded = taker.remaining().min(maker.remaining());
        assert_eq!(traded, taker.can_trade(&maker));

        let trade = Trade::new(&mut taker, &mut maker, traded);
        assert!(trade.is_ok());

        // check that the orders have been filled correctly
        assert_eq!(taker.remaining(), 5.into());
        assert_eq!(maker.remaining(), OrderQuantity::ZERO);

        // check the details of the trade
        let trade = trade.unwrap();
        assert_eq!(trade.taker, taker.id());
        assert_eq!(trade.maker, maker.id());
        assert_eq!(trade.quantity, traded);
        assert_eq!(trade.price, maker.limit_price().unwrap());
    }
}
