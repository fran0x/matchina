use compact_str::CompactString;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum OrderSide {
    Ask,
    Bid,
}

impl OrderSide {
    #[inline]
    fn _toggle(&self) -> Self {
        match self {
            Self::Ask => Self::Bid,
            Self::Bid => Self::Ask,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE", tag = "order_request")]
pub enum OrderRequest {
    Create {
        account_id: CompactString,
        order_id: CompactString,
        pair: CompactString,
        quantity: Decimal,
        price: Decimal,
        side: OrderSide,
    },
    Cancel {
        order_id: CompactString,
    },
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrderId(u64);

impl OrderId {
    #[inline]
    pub fn new(order_id: u64) -> Self {
        Self(order_id)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE", tag = "order_type")]
pub enum OrderType {
    Limit {
        limit_price: u64,
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
    #[serde(flatten)]
    type_: OrderType,
    quantity: u64,
    #[serde(default)]
    filled: u64,
    status: OrderStatus,
}

pub mod util {
    use compact_str::{format_compact, CompactString};
    use rand::Rng;

    use super::{OrderRequest, OrderSide};
    
    pub const DEFAULT_PAIR: &'static str = "ETH/USDT";

    pub fn generate(range: impl Iterator<Item = usize>) -> impl Iterator<Item = OrderRequest> {
        let mut rng = rand::thread_rng();

        let orders = range.map(move |i| {
            if rng.gen_bool(1.0 / 1000.0) {
                OrderRequest::Cancel {
                    order_id: format_compact!("{}", rng.gen_range(1..=i as u64)),
                }
            } else {
                OrderRequest::Create {
                    account_id: format_compact!("{}", rng.gen_range(1..100)),
                    order_id: format_compact!("{}", i as u64),
                    pair: CompactString::new_inline(DEFAULT_PAIR),
                    quantity: rng.gen_range(100..10_000).into(),
                    price: rng.gen_range(100..10_000).into(),
                    side: if rng.gen_bool(0.5) {
                        OrderSide::Ask
                    } else {
                        OrderSide::Bid
                    },
                }
            }
        });

        orders
    }
}
