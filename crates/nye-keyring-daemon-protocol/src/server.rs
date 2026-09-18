use anyhow::Context;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::messages::{ClientMessage, ServerMessage};

pub async fn send<S, M>(
    stream: &mut S,
    message_id: u64,
    message_contents: M,
) -> anyhow::Result<()>
where
    S: AsyncWrite + Unpin,
    M: Into<ServerMessage>,
{
    let message_contents = bitcode::encode(&message_contents.into());
    let message_size = message_contents.len() as u64;

    stream
        .write_u64(message_id)
        .await
        .context("Could not send message ID.")?;
    stream
        .write_u64(message_size)
        .await
        .context("Coudl not send message size.")?;
    stream
        .write_all(&message_contents)
        .await
        .context("Could not send message contents.")?;

    Ok(())
}

pub async fn recv<S>(stream: &mut S) -> anyhow::Result<(u64, ClientMessage)>
where
    S: AsyncRead + Unpin,
{
    let message_id = stream
        .read_u64()
        .await
        .context("Could not read ID of next message.")?;
    let message_size = stream
        .read_u64()
        .await
        .context("Could not read size of next message.")?;
    let message_contents = {
        let mut buffer = vec![0u8; message_size as usize];
        stream
            .read_exact(&mut buffer)
            .await
            .context("Could not read next message's contents.")?;
        buffer
    };

    let message: ClientMessage =
        bitcode::decode(&message_contents).context("Could not decode next message.")?;

    Ok((message_id, message))
}
