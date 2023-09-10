use anyhow::Result;
use compact_str::CompactString;
use thiserror::Error;

use crate::order::OrderRequest;

pub struct Engine {
    _pair: CompactString,
}

impl Engine {
    #[inline]
    pub fn new(pair: &str) -> Self {
        Self {
            _pair: CompactString::new_inline(pair),
        }
    }

    #[inline]
    pub fn process(&mut self, _order_request: OrderRequest) -> Result<(), EngineError> {
        Ok(())
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
