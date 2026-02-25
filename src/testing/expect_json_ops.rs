//! This contains helpers used in our tests.

use crate::expect_json::expect_core::Context;
use crate::expect_json::expect_core::ExpectOp;
use crate::expect_json::expect_core::ExpectOpResult;

// This needs to be the external crate, as the `::axum_test` path doesn't work within our tests.
use ::expect_json::expect_core::expect_op;

#[expect_op]
#[derive(Clone, Debug)]
pub struct ExpectStrMinLen {
    pub min: usize,
}

impl ExpectOp for ExpectStrMinLen {
    fn on_string(&self, _context: &mut Context<'_>, received: &str) -> ExpectOpResult<()> {
        if received.len() < self.min {
            panic!("String is too short, received: {received}");
        }

        Ok(())
    }
}
