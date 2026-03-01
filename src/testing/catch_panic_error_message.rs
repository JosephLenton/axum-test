use futures::FutureExt;
use std::fmt::Debug;
use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;

pub fn catch_panic_error_message<F>(func: F) -> String
where
    F: FnOnce() -> (),
{
    catch_unwind(AssertUnwindSafe(func))
        .unwrap_err()
        .downcast_ref::<String>()
        .unwrap()
        .to_owned()
}

pub async fn catch_panic_error_message_async<Fut, T>(fut: Fut) -> String
where
    Fut: IntoFuture<Output = T>,
    T: Debug,
{
    AssertUnwindSafe(fut.into_future())
        .catch_unwind()
        .await
        .unwrap_err()
        .downcast_ref::<String>()
        .unwrap()
        .to_owned()
}
