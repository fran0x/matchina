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

#[cfg(test)]
mod test {
    use crate::{
        order::{Order, OrderSide},
        trade::Trade,
    };

    #[test]
    fn test_trade_new() {
        // create two mock limit orders with matching prices
        let taker_id = 1;
        let maker_id = 2;
        let taker = Order::limit_order(taker_id, OrderSide::Bid, 100, 15);
        let maker = Order::limit_order(maker_id, OrderSide::Ask, 100, 10);

        // call Trade::new and expect it to succeed
        let result = Trade::new(&mut taker.clone(), &mut maker.clone());
        assert!(result.is_ok());

        // check that the orders have been filled correctly
        assert_eq!(taker.remaining(), 5);
        assert_eq!(maker.remaining(), 0);

        // check the details of the trade
        let trade = result.unwrap();
        assert_eq!(trade.taker, taker_id);
        assert_eq!(trade.maker, maker_id);
        assert_eq!(trade.quantity, 10);
        assert_eq!(trade.price, 100);
    }
}
