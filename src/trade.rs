use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::order::{LimitOrder, OrderError, OrderId, OrderPrice, OrderQuantity, Order};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Trade {
    taker: OrderId,
    maker: OrderId,
    price: OrderPrice,
    quantity: OrderQuantity,
}

impl Trade {
    #[inline]
    pub fn new(taker: &mut Order, maker: &mut LimitOrder) -> Result<Trade, TradeError> {
        if !taker.matches(maker) {
            Err(TradeError::PriceNotMatching)?;
        }

        let traded = taker.remaining().min(maker.remaining());
        let price = maker.limit_price();

        taker.fill(traded)?;
        maker.fill(traded)?;

        Ok(Trade {
            taker: taker.id(),
            maker: maker.id(),
            price,
            quantity: traded,
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

#[cfg(test)]
mod test {
    use crate::{
        order::{LimitOrder, OrderId, OrderSide},
        trade::Trade,
    };

    #[test]
    fn match_limit_orders() {
        // create two mock limit orders with matching prices
        let taker_id: OrderId = 1.into();
        let maker_id: OrderId = 2.into();
        let mut taker = LimitOrder::new(taker_id, OrderSide::Bid, 100.into(), 15.into());
        let mut maker = LimitOrder::new(maker_id, OrderSide::Ask, 100.into(), 10.into());

        // call Trade::new and expect it to succeed
        let result = Trade::new(&mut taker, &mut maker);
        assert!(result.is_ok());

        // check that the orders have been filled correctly
        assert_eq!(taker.remaining(), 5.into());
        assert_eq!(maker.remaining(), 0.into());

        // check the details of the trade
        let trade = result.unwrap();
        assert_eq!(trade.taker, taker_id);
        assert_eq!(trade.maker, maker_id);
        assert_eq!(trade.quantity, 10.into());
        assert_eq!(trade.price, 100.into());
    }
}
