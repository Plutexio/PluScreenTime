//! The socket the app asks over: a line of JSON in, a line out.

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use anyhow::{Context, Result, bail};
use pluscreentime_protocol::{Request, error_line};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{mpsc, oneshot};
use tracing::debug;

/// A request on its way to the tracker, which owns the history.
pub struct Query {
    pub request: Request,
    pub reply: oneshot::Sender<String>,
}

/// Longer lines are no request.
const MAX_LINE: usize = 64 * 1024;

pub fn bind(path: &Path) -> Result<UnixListener> {
    if path.exists() {
        if std::os::unix::net::UnixStream::connect(path).is_ok() {
            bail!("another pluscreentimed is already running on {}", path.display());
        }
        std::fs::remove_file(path).with_context(|| format!("removing the stale {}", path.display()))?;
    }
    let listener = UnixListener::bind(path).with_context(|| format!("binding {}", path.display()))?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(listener)
}

pub async fn serve(listener: UnixListener, queries: mpsc::Sender<Query>) -> Result<()> {
    loop {
        let (stream, _) = listener.accept().await?;
        let queries = queries.clone();
        tokio::spawn(async move {
            if let Err(e) = client(stream, queries).await {
                debug!("client: {e:#}");
            }
        });
    }
}

async fn client(stream: UnixStream, queries: mpsc::Sender<Query>) -> Result<()> {
    let (read, mut write) = stream.into_split();
    let mut lines = BufReader::new(read).lines();
    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        let answer = if line.len() > MAX_LINE {
            error_line("request too long")
        } else {
            match serde_json::from_str::<Request>(&line) {
                Ok(request) => {
                    let (reply, answer) = oneshot::channel();
                    queries.send(Query { request, reply }).await?;
                    answer.await?
                }
                Err(e) => error_line(&format!("malformed request: {e}")),
            }
        };
        write.write_all(answer.as_bytes()).await?;
        write.write_all(b"\n").await?;
    }
    Ok(())
}
