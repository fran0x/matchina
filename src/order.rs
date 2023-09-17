use std::{cmp::Ordering, fmt::Display};

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

impl Display for OrderRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderRequest::Create {
                account_id: _,
                order_id,
                pair: _,
                side,
                limit_price,
                quantity,
            } => match limit_price {
                Some(limit_price) => write!(f, "ORDER[{order_id}] {side} {quantity}@{limit_price}"),
                None => write!(f, "ORDER[{order_id}] {side} {quantity}@MARKET"),
            },
            OrderRequest::Cancel { order_id } => write!(f, "[CANCEL] order_id: {order_id}"),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum OrderSide {
    Ask,
    Bid,
}

impl Display for OrderSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderSide::Ask => write!(f, "SELL"),
            OrderSide::Bid => write!(f, "BUY"),
        }
    }
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
            OrderType::Limit { limit_price, .. } => Some(limit_price),
            OrderType::Market { .. } => None,
        }
    }

    #[inline]
    pub fn is_bookable(&self) -> bool {
        match self.type_ {
            OrderType::Limit { .. } => true,
            OrderType::Market { .. } => false,
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

        match taker.type_ {
            OrderType::Limit { .. } => match (taker.side(), maker.side()) {
                (OrderSide::Ask, OrderSide::Bid) => taker <= maker,
                (OrderSide::Bid, OrderSide::Ask) => taker >= maker,
                _ => false,
            },
            OrderType::Market { .. } => true,
        }
    }

    #[inline]
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

    #[inline]
    pub fn cancel(&mut self) {
        match self.status() {
            OrderStatus::Open => self.status = OrderStatus::Cancelled,
            OrderStatus::Partial => self.status = OrderStatus::Closed,
            _ => (),
        }
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

pub trait Flags {
    fn is_all_or_none(&self) -> bool;

    fn is_immediate_or_cancel(&self) -> bool;

    fn is_post_only(&self) -> bool;
}

impl Flags for Order {
    #[inline]
    fn is_all_or_none(&self) -> bool {
        match self.type_ {
            OrderType::Market { all_or_none }
            | OrderType::Limit {
                time_in_force: TimeInForce::ImmediateOrCancel { all_or_none },
                ..
            } => all_or_none,
            _ => false,
        }
    }

    #[inline]
    fn is_immediate_or_cancel(&self) -> bool {
        matches!(
            self.type_,
            OrderType::Limit {
                time_in_force: TimeInForce::ImmediateOrCancel { .. },
                ..
            } | OrderType::Market { .. }
        )
    }

    #[inline]
    fn is_post_only(&self) -> bool {
        matches!(self.type_, OrderType::Limit { time_in_force: TimeInForce::GoodTilCancel { post_only }, .. } if post_only)
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
    use rand::{rngs::ThreadRng, Rng};
    use rust_decimal::Decimal;

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
                    account_id: format_compact!("{}", rng.gen_range(1..10)),
                    order_id: i as u64,
                    pair: CompactString::new_inline(DEFAULT_PAIR),
                    side: if rng.gen_bool(0.5) {
                        OrderSide::Ask
                    } else {
                        OrderSide::Bid
                    },
                    limit_price: if rng.gen_bool(0.8) {
                        Some(random_decimal(&mut rng))
                    } else {
                        None
                    },
                    quantity: random_decimal(&mut rng),
                }
            }
        })
    }

    pub fn random_decimal(rng: &mut ThreadRng) -> Decimal {
        Decimal::new(rng.gen_range(10000..1_000_000), 2)
    }
}
