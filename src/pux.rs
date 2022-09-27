use futures_util::future::try_join_all;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::SendError;
use tokio::sync::broadcast::{Receiver, Sender};

use crate::{Entrypoint, PuxResult};

pub struct Pux {
  entrypoints: Vec<Entrypoint>,
  shutdown_tx: Sender<()>,
  shutdown_rx: Receiver<()>,
}

impl Pux {
  pub fn new(entrypoints: Vec<Entrypoint>) -> Self {
    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    Self {
      entrypoints,
      shutdown_tx,
      shutdown_rx,
    }
  }

  pub fn shutdown(&self) -> Result<(), SendError<()>> {
    self.shutdown_tx.send(())?;
    Ok(())
  }

  pub async fn start(&self) -> PuxResult<()> {
    let mut listeners = Vec::with_capacity(self.entrypoints.len());

    for entrypoint in &self.entrypoints {
      listeners.push(entrypoint.accept(self.shutdown_tx.subscribe()))
    }

    try_join_all(listeners).await?;

    Ok(())
  }
}
