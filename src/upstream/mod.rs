use std::net::SocketAddr;

use hyper::{Body, Request, Response};
use tokio_rustls::rustls::ServerName;

pub use crate::error::*;
use crate::upstream::pool::HttpPool;
use crate::PuxResult;

mod conn;
mod error;
mod pool;

pub struct Upstream {
  pool: HttpPool,
}

impl Upstream {
  pub async fn new(addrs: Vec<SocketAddr>, sni: Option<ServerName>) -> Self {
    Self {
      pool: HttpPool::new(addrs, sni),
    }
  }

  pub async fn send(&self, req: Request<Body>) -> PuxResult<Response<Body>> {
    Ok(self.pool.send(req).await.unwrap())
  }
}
