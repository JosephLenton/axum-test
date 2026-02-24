use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;
use tokio::runtime::Handle;

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

pub fn catch_panic_error_message_async<F, Fut>(func: F) -> String
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = ()>,
{
    catch_panic_error_message(|| {
        Handle::current().block_on(async {
            func().await;
        })
    })
}
