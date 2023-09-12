use std::{
    cmp::Reverse,
    collections::{BTreeMap, VecDeque},
    marker::PhantomData,
};

use indexmap::IndexMap;

use crate::{
    order::{Order, OrderId, OrderPrice},
    trade::Trade,
};

pub struct Orderbook {
    _orders: IndexMap<OrderId, Order>,
    _asks: BTreeMap<OrderPrice, VecDeque<OrderId>>,
    _bids: BTreeMap<Reverse<OrderPrice>, VecDeque<OrderId>>,
    _trade: PhantomData<Trade>,
}
