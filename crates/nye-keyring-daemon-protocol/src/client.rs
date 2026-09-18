use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Context;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio::task::JoinHandle;

use crate::messages::{ClientMessage, ServerMessage};

type Query = (ClientMessage, oneshot::Sender<ServerMessage>);

pub struct ProtocolClient {
    router: JoinHandle<anyhow::Result<()>>,
    outgoing: mpsc::Sender<Query>,
}

impl ProtocolClient {
    pub async fn connect() -> anyhow::Result<Self> {
        let stream = UnixStream::connect("/run/nye-keyring-daemon.sock")
            .await
            .context("Could not connect to nye keyring daemon at /run/nye-keyring-daemon.sock.")?;

        let (tx, rx) = mpsc::channel(128);

        let router = tokio::spawn(Self::route(stream, rx));

        Ok(ProtocolClient {
            router,
            outgoing: tx,
        })
    }

    async fn route(stream: UnixStream, outgoing: mpsc::Receiver<Query>) -> anyhow::Result<()> {
        let (rx, tx) = stream.into_split();
        let queries = Arc::new(Mutex::new(HashMap::new()));

        let task_incoming = Self::route_incoming(rx, queries.clone());
        let task_outgoing = Self::route_outgoing(tx, queries.clone(), outgoing);

        tokio::select! {
            result = task_outgoing => {
                result.context("The outgoing messages task failed.")?;
            },
            result = task_incoming => {
                result.context("The incoming messages task failed.")?;
            }
        };

        // Will never run.
        Ok(())
    }

    async fn route_outgoing(
        mut tx: OwnedWriteHalf,
        queries: Arc<Mutex<HashMap<u64, oneshot::Sender<ServerMessage>>>>,
        mut outgoing: mpsc::Receiver<Query>,
    ) -> anyhow::Result<()> {
        let mut message_id = 0;

        loop {
            let (message, response) = outgoing
                .recv()
                .await
                .context("Could not receive the next outgoing message.")?;

            queries.lock().await.insert(message_id, response);

            let message_contents = bitcode::encode(&message);
            let message_size = message_contents.len() as u64;

            tx.write_u64(message_id)
                .await
                .context("Could not send message ID.")?;
            tx.write_u64(message_size)
                .await
                .context("Coudl not send message size.")?;
            tx.write_all(&message_contents)
                .await
                .context("Could not send message contents.")?;

            message_id += 1;
        }
    }

    async fn route_incoming(
        mut rx: OwnedReadHalf,
        queries: Arc<Mutex<HashMap<u64, oneshot::Sender<ServerMessage>>>>,
    ) -> anyhow::Result<()> {
        loop {
            let message_id = rx
                .read_u64()
                .await
                .context("Could not read ID of next message.")?;
            let message_size = rx
                .read_u64()
                .await
                .context("Could not read size of next message.")?;
            let message_content = {
                let mut buffer = vec![0u8; message_size as usize];
                rx.read_exact(&mut buffer)
                    .await
                    .context("Could not read next message's contents.")?;
                buffer
            };

            let message: ServerMessage =
                bitcode::decode(&message_content).context("Could not decode next message.")?;

            // Messages that get sent with an ID that doesn't match any waiters get ignored.
            if let Some(response) = queries.lock().await.remove(&message_id) {
                let _ = response.send(message);
            }
        }
    }

    pub async fn query(
        &mut self,
        message: impl Into<ClientMessage>,
    ) -> anyhow::Result<ServerMessage> {
        let (tx, rx) = oneshot::channel();

        self.outgoing
            .send((message.into(), tx))
            .await
            .context("Could not send outgoing message through channel. Did the router crash?")?;

        rx.await.context("Could not receive message from server.")
    }
}

impl Drop for ProtocolClient {
    fn drop(&mut self) {
        self.router.abort();
    }
}
