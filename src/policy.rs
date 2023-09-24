use crate::{
    order::Flags,
    order::Order,
    orderbook::{Orderbook, Scanner},
};

pub trait Policy {
    fn enforce(&self, order: &mut Order, orderbook: &Orderbook);
}

pub struct AllOrNone;

impl Policy for AllOrNone {
    #[inline]
    fn enforce(&self, incoming_order: &mut Order, exchange: &Orderbook) {
        if incoming_order.is_all_or_none()
            && incoming_order.remaining()
                > exchange.volume_with(incoming_order.side().opposite(), |order| order.matches(&incoming_order))
        {
            incoming_order.cancel();
        }
    }
}

pub struct PostOnly;

impl Policy for PostOnly {
    #[inline]
    fn enforce(&self, order: &mut Order, orderbook: &Orderbook) {
        if order.is_post_only()
            && !orderbook
                .peek(&!order.side())
                .is_some_and(|top_order| order.matches(top_order))
        {
            order.cancel();
        }
    }
}

pub struct ImmediateOrCancel;

impl Policy for ImmediateOrCancel {
    #[inline]
    fn enforce(&self, order: &mut Order, orderbook: &Orderbook) {
        if order.is_immediate_or_cancel() {
            order.cancel();
        }
    }
}
