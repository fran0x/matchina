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
        order::{Order, OrderId, OrderSide},
        trade::Trade,
    };

    #[test]
    fn test_match() {
        // create two mock limit orders with matching prices
        let taker_id: OrderId = 1.into();
        let maker_id: OrderId = 2.into();
        let mut taker = Order::limit_order(taker_id, OrderSide::Bid, 15.into(), 100.into());
        let mut maker = Order::limit_order(maker_id, OrderSide::Ask, 10.into(), 100.into());

        // call Trade::new and expect it to succeed
        let quantity = taker.can_trade(&maker);
        let trade = Trade::new(&mut taker, &mut maker, quantity);
        assert!(trade.is_ok());

        // check that the orders have been filled correctly
        assert_eq!(taker.remaining(), 5.into());
        assert_eq!(maker.remaining(), 0.into());

        // check the details of the trade
        let trade = trade.unwrap();
        assert_eq!(trade.taker, taker_id);
        assert_eq!(trade.maker, maker_id);
        assert_eq!(trade.quantity, 10.into());
        assert_eq!(trade.price, 100.into());
    }
}
