use futures_util::future::try_join_all;
use tokio::sync::broadcast::error::SendError;

use crate::{Entrypoint, PuxResult};

pub(crate) struct Pux {
  entrypoints: Vec<Entrypoint>,
}

impl Pux {
  pub(crate) fn new(entrypoints: Vec<Entrypoint>) -> Self {
    Self { entrypoints }
  }

  pub(crate) fn shutdown(&self) -> Result<(), SendError<()>> {
    Ok(())
  }

  pub(crate) async fn start(&self) -> PuxResult<()> {
    let mut listeners = Vec::with_capacity(self.entrypoints.len());

    for entrypoint in &self.entrypoints {
      listeners.push(entrypoint.accept())
    }

    try_join_all(listeners).await?;

    Ok(())
  }
}
