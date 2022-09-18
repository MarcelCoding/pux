use futures_util::future::try_join_all;

use crate::{Entrypoint, PuxResult};

pub struct Pux {
  entrypoints: Vec<Entrypoint>,
}

impl Pux {
  pub fn new(entrypoints: Vec<Entrypoint>) -> Self {
    Self { entrypoints }
  }

  pub async fn start(&self) -> PuxResult<()> {
    let mut listeners = Vec::with_capacity(self.entrypoints.len());

    for entrypoint in &self.entrypoints {
      listeners.push(entrypoint.accept())
    }

    try_join_all(listeners).await?;

    Ok(())
  }
}
