use tokio::task::JoinHandle;

/// A handle to a running Axum service.
///
/// When the handle is dropped, it will attempt to terminate the service.
#[derive(Debug)]
pub struct ServeHandle {
    server_handle: JoinHandle<()>,
}

impl ServeHandle {
    pub(crate) fn new(server_handle: JoinHandle<()>) -> Self {
        Self { server_handle }
    }

    pub fn is_finished(&self) -> bool {
        self.server_handle.is_finished()
    }
}

impl Drop for ServeHandle {
    fn drop(&mut self) {
        self.server_handle.abort()
    }
}
