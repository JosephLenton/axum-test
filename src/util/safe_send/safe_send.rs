use anyhow::Result;
use std::sync::Arc;
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::sync::mpsc::SyncSender;
use std::thread::JoinHandle;

#[derive(Debug, Clone)]
pub(crate) struct SafeSend<In, Out> {
    task_sender: SyncSender<(In, Sender<Out>)>,
    thread: Arc<JoinHandle<()>>,
}

impl<In, Out> SafeSend<In, Out> {
    pub(crate) fn new(
        task_sender: SyncSender<(In, Sender<Out>)>,
        thread: Arc<JoinHandle<()>>,
    ) -> Self {
        Self {
            task_sender,
            thread,
        }
    }
}

impl<In, Out> SafeSend<In, Out> {
    pub(crate) fn is_running(&self) -> bool {
        !self.thread.is_finished()
    }
}

impl<In, Out> SafeSend<In, Out>
where
    In: Send + 'static,
    Out: Send + 'static,
{
    pub(crate) async fn send(&self, input: In) -> Result<Out> {
        let task_sender = self.task_sender.clone();

        tokio::task::spawn_blocking(move || {
            let (response_tx, response_rx) = mpsc::channel::<Out>();

            task_sender
                .send((input, response_tx))
                .map_err(|_| anyhow::anyhow!("SafeSend background thread has stopped"))?;

            response_rx
                .recv()
                .map_err(|_| anyhow::anyhow!("SafeSend background thread dropped response sender"))
        })
        .await
        .map_err(|e| anyhow::anyhow!("SafeSend background thread panicked: {e}"))?
    }
}
