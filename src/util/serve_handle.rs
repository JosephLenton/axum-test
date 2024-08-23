use ::std::sync::Arc;
use ::tokio::sync::Notify;
use ::tokio::task::JoinHandle;

#[derive(Debug)]
pub struct ServeHandle {
    server_handle: JoinHandle<()>,
    shutdown_notification: Arc<Notify>,
}

impl ServeHandle {
    pub(crate) fn new(server_handle: JoinHandle<()>, shutdown_notification: Arc<Notify>) -> Self {
        Self {
            server_handle,
            shutdown_notification,
        }
    }
}

impl Drop for ServeHandle {
    fn drop(&mut self) {
        self.shutdown_notification.notify_waiters();
        self.server_handle.abort()
    }
}
