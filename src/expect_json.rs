pub use ::expect_json::expect::*;

/// For implementing your own expectations.
pub mod expect_core {
    pub use ::expect_json::expect_core::*;

    /// This macro is for defining your own custom [`ExpectOp`] checks.
    #[doc(inline)]
    pub use ::expect_json::expect_core::expect_op_for_axum_test as expect_op;

    pub use ::expect_json::ExpectJsonError;
    pub use ::expect_json::ExpectJsonResult;
    pub use ::expect_json::JsonType;
}

#[doc(hidden)]
pub use ::expect_json::__private;
