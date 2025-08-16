use axum_test::expect_json::expect_core::expect_op;
use axum_test::expect_json::expect_core::ExpectOp;

// If it compiles, it works!
#[test]
fn test_expect_op_for_axum_test_integration_compiles() {
    #[expect_op]
    #[derive(Debug, Clone)]
    pub struct Testing;

    impl ExpectOp for Testing {}

    assert!(true);
}
