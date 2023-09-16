use std::cmp::Ordering;

use compact_str::CompactString;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::trade::Trade;

#[derive(Clone, Copy, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrderId(u64);

impl OrderId {
    #[inline]
    pub fn new(order_id: u64) -> Self {
        Self(order_id)
    }
}

impl From<u64> for OrderId {
    fn from(value: u64) -> OrderId {
        OrderId::new(value)
    }
}

// TODO use struct to give behavior (see OrderId)
pub type OrderPrice = Decimal;
pub type OrderQuantity = Decimal;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE", tag = "order_request")]
pub enum OrderRequest {
    Create {
        account_id: CompactString,
        order_id: u64,
        pair: CompactString,
        side: OrderSide,
        limit_price: Option<Decimal>, // for market orders use None
        quantity: Decimal,
    },
    Cancel {
        order_id: u64,
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum OrderSide {
    Ask,
    Bid,
}

impl std::ops::Not for OrderSide {
    type Output = OrderSide;

    #[inline]
    fn not(self) -> Self::Output {
        match self {
            Self::Ask => Self::Bid,
            Self::Bid => Self::Ask,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE", tag = "order_type")]
pub enum OrderType {
    Limit {
        limit_price: OrderPrice,
        #[serde(default)]
        time_in_force: TimeInForce,
    },

    Market {
        #[serde(default, skip_serializing_if = "core::ops::Not::not")]
        all_or_none: bool,
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE", tag = "time_in_force")]
pub enum TimeInForce {
    #[serde(rename = "GTC")]
    GoodTilCancel {
        #[serde(default, skip_serializing_if = "core::ops::Not::not")]
        post_only: bool,
    },
    #[serde(rename = "IOC")]
    ImmediateOrCancel {
        #[serde(default, skip_serializing_if = "core::ops::Not::not")]
        all_or_none: bool,
    },
}

impl Default for TimeInForce {
    fn default() -> Self {
        Self::GoodTilCancel { post_only: false }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum OrderStatus {
    #[default]
    Open,
    Partial,
    Cancelled,
    Closed,
    Completed,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Order {
    id: OrderId,
    side: OrderSide,
    //#[serde(flatten)]
    type_: OrderType,
    order_quantity: OrderQuantity,
    //#[serde(default)]
    filled_quantity: OrderQuantity,
    status: OrderStatus,
}

impl Order {
    #[inline]
    pub fn limit_order(id: OrderId, side: OrderSide, limit_price: OrderPrice, quantity: OrderQuantity) -> Self {
        Self {
            id,
            side,
            type_: OrderType::Limit {
                limit_price,
                time_in_force: Default::default(),
            },
            order_quantity: quantity,
            filled_quantity: 0.into(),
            status: OrderStatus::Open,
        }
    }

    #[inline]
    pub fn market_order(id: OrderId, side: OrderSide, quantity: OrderQuantity) -> Self {
        Self {
            id,
            side,
            type_: OrderType::Market {
                all_or_none: Default::default(),
            },
            order_quantity: quantity,
            filled_quantity: 0.into(),
            status: OrderStatus::Open,
        }
    }

    #[inline]
    pub fn id(&self) -> OrderId {
        self.id
    }

    #[inline]
    pub fn side(&self) -> OrderSide {
        self.side
    }

    #[inline]
    pub fn remaining(&self) -> OrderQuantity {
        self.order_quantity - self.filled_quantity
    }

    #[inline]
    fn status(&self) -> OrderStatus {
        self.status
    }

    #[inline]
    pub fn limit_price(&self) -> Option<OrderPrice> {
        match self.type_ {
            OrderType::Limit {
                limit_price,
                time_in_force: _,
            } => Some(limit_price),
            OrderType::Market { all_or_none: _ } => None,
        }
    }

    #[inline]
    pub fn is_closed(&self) -> bool {
        matches!(
            self.status(),
            OrderStatus::Cancelled | OrderStatus::Closed | OrderStatus::Completed
        )
    }

    #[inline]
    pub fn trade(&mut self, other: &mut Self) -> Option<Trade> {
        let (taker, maker) = (self, other);
        Trade::new(taker, maker).ok()
    }

    #[inline]
    pub fn matches(&self, maker: &Self) -> bool {
        let taker = self;

        if taker.is_closed() || maker.is_closed() {
            return false;
        }

        match (taker.side(), maker.side()) {
            (OrderSide::Ask, OrderSide::Bid) => matches!(taker.type_, OrderType::Market { .. }) || taker <= maker,
            (OrderSide::Bid, OrderSide::Ask) => matches!(taker.type_, OrderType::Market { .. }) || taker >= maker,
            _ => false,
        }
    }

    pub fn fill(&mut self, quantity: OrderQuantity) -> Result<(), OrderError> {
        if quantity > self.remaining() {
            return Err(OrderError::Overfill {
                fill: quantity,
                remaining: self.remaining(),
            });
        }

        self.filled_quantity += quantity;
        self.status = if self.filled_quantity == self.order_quantity {
            OrderStatus::Completed
        } else {
            OrderStatus::Partial
        };

        Ok(())
    }
}

impl PartialEq for Order {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}
impl Eq for Order {}

impl PartialOrd for Order {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let ord = if self.id.eq(&other.id) {
            Ordering::Equal
        } else {
            self.limit_price().cmp(&other.limit_price())
        };

        Some(ord)
    }
}

#[derive(Debug, Error)]
pub enum OrderError {
    #[error("fill exceeds remaning amount (fill={}, remaining={})", .fill, .remaining)]
    Overfill {
        fill: OrderQuantity,
        remaining: OrderQuantity,
    },
}

pub mod util {
    use compact_str::{format_compact, CompactString};
    use rand::Rng;

    use super::{OrderRequest, OrderSide};

    pub const DEFAULT_PAIR: &str = "ETH/USDT";

    pub fn generate(range: impl Iterator<Item = usize>) -> impl Iterator<Item = OrderRequest> {
        let mut rng = rand::thread_rng();

        range.map(move |i| {
            if rng.gen_bool(1.0 / 1000.0) {
                OrderRequest::Cancel {
                    order_id: rng.gen_range(1..=i as u64),
                }
            } else {
                OrderRequest::Create {
                    account_id: format_compact!("{}", rng.gen_range(1..100)),
                    order_id: i as u64,
                    pair: CompactString::new_inline(DEFAULT_PAIR),
                    side: if rng.gen_bool(0.5) {
                        OrderSide::Ask
                    } else {
                        OrderSide::Bid
                    },
                    limit_price: if rng.gen_bool(0.8) {
                        Some(rng.gen_range(100..10_000).into())
                    } else {
                        None
                    },
                    quantity: rng.gen_range(100..10_000).into(),
                }
            }
        })
    }
}
