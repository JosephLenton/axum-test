use ::anyhow::anyhow;
use ::anyhow::Result;
use ::std::sync::Arc;
use ::std::sync::Mutex;

pub fn with_this_mut<T, F, R>(this: &mut Arc<Mutex<T>>, name: &str, some_action: F) -> Result<R>
where
    F: FnOnce(&mut T) -> R,
{
    let mut this_locked = this.lock().map_err(|err| {
        anyhow!(
            "Failed to lock InternalTestServer for `{}`, {:?}",
            name,
            err,
        )
    })?;

    let result = some_action(&mut this_locked);

    Ok(result)
}
