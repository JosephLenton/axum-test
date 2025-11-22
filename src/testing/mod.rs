//! This contains helpers used in our tests.

#[cfg(not(feature = "old-json-diff"))]
use crate::expect_json::expect_core::Context;
#[cfg(not(feature = "old-json-diff"))]
use crate::expect_json::expect_core::ExpectOp;
#[cfg(not(feature = "old-json-diff"))]
use crate::expect_json::expect_core::ExpectOpResult;

// This needs to be the external crate, as the `::axum_test` path doesn't work within our tests.
#[cfg(not(feature = "old-json-diff"))]
use ::expect_json::expect_core::expect_op;

#[cfg(not(feature = "old-json-diff"))]
#[expect_op]
#[derive(Clone, Debug)]
pub struct ExpectStrMinLen {
    pub min: usize,
}

#[cfg(not(feature = "old-json-diff"))]
impl ExpectOp for ExpectStrMinLen {
    fn on_string(&self, _context: &mut Context<'_>, received: &str) -> ExpectOpResult<()> {
        if received.len() < self.min {
            panic!("String is too short, received: {received}");
        }

        Ok(())
    }
}
