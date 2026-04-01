use crate::util::SafeSend;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::sync::mpsc::sync_channel;
use std::thread::spawn;
use tokio::runtime::Builder;
use tokio::task::LocalSet;

#[derive(Debug)]
pub(crate) struct SafeSendBuilder<F> {
    init: F,
}

impl<F, Fut, S> SafeSendBuilder<F>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = S> + 'static,
    S: 'static,
{
    pub(crate) fn new(init: F) -> Self {
        Self { init }
    }

    pub(crate) fn on_send<G, In, Out>(self, handler: G) -> SafeSend<In, Out>
    where
        G: for<'s> Fn(&'s S, In) -> Pin<Box<dyn Future<Output = Out> + 's>> + Send + 'static,
        In: Send + 'static,
        Out: Send + 'static,
    {
        let (task_tx, task_rx) = sync_channel::<(In, Sender<Out>)>(0);

        let thread = spawn(move || {
            let rt = Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to build tokio runtime for SafeSend");

            let local = LocalSet::new();

            let service = rt.block_on(local.run_until(async { (self.init)().await }));

            while let Ok((input, response_tx)) = task_rx.recv() {
                let output = rt.block_on(local.run_until(handler(&service, input)));
                response_tx.send(output).expect("Failed to send reply");
            }
        });

        SafeSend::new(task_tx, Arc::new(thread))
    }
}
