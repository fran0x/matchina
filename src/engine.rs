use anyhow::Result;
use compact_str::CompactString;
use thiserror::Error;

use crate::{
    order::{Order, OrderRequest},
    orderbook::Orderbook,
};

pub struct Engine {
    _pair: CompactString,
    orderbook: Orderbook,
}

impl Engine {
    #[inline]
    pub fn new(pair: &str) -> Self {
        Self {
            _pair: CompactString::new_inline(pair),
            orderbook: Orderbook::default(),
        }
    }

    #[inline]
    pub fn process(&mut self, order_request: OrderRequest) -> Result<(), EngineError> {
        //info!("{order_request}");
        match order_request {
            OrderRequest::Create {
                account_id: _,
                order_id,
                pair: _,
                side,
                limit_price,
                quantity,
            } => {
                let order = if let Some(limit_price) = limit_price {
                    Order::limit_order(order_id.into(), side, limit_price, quantity)
                } else {
                    Order::market_order(order_id.into(), side, quantity)
                };
                let _ = self.orderbook.handle_create(order);
            }
            OrderRequest::Cancel { order_id } => {
                let _ = self.orderbook.handle_cancel(order_id.into());
            }
        };

        Ok(())
    }

    #[inline]
    pub fn orderbook(&self) -> &Orderbook {
        &self.orderbook
    }
}

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("invalid pair (expected={}, found={})", .expected, .found)]
    InvalidPair {
        expected: CompactString,
        found: CompactString,
    },
}
