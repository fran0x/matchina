use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::order::{Order, OrderError, OrderId, OrderPrice, OrderQuantity};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Trade {
    taker: OrderId,
    maker: OrderId,
    quantity: OrderQuantity,
    price: OrderPrice,
}

impl Trade {
    #[inline]
    pub fn new(taker: &mut Order, maker: &mut Order) -> Result<Trade, TradeError> {
        if !taker.matches(maker) {
            Err(TradeError::PriceNotMatching)?;
        }

        let traded = taker.remaining().min(maker.remaining());
        let price = maker.limit_price().expect("maker must always have a price");

        taker.fill(traded)?;
        maker.fill(traded)?;

        Ok(Trade {
            taker: taker.id(),
            maker: maker.id(),
            quantity: traded,
            price,
        })
    }

    #[inline]
    pub fn price(&self) -> OrderPrice {
        self.price
    }
}

#[derive(Debug, Error)]
pub enum TradeError {
    #[error("prices do not match each other")]
    PriceNotMatching,
    #[error("order error: {0}")]
    OrderError(#[from] OrderError),
}
