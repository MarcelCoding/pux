use std::net::SocketAddr;

use hyper::{Body, Request, Response};
use tokio_rustls::rustls::ServerName;

use crate::upstream::pool::HttpPool;
use crate::PuxResult;

mod conn;
mod error;
mod pool;

pub(crate) struct Upstream {
  pool: HttpPool,
}

impl Upstream {
  pub(crate) async fn new(addrs: Vec<SocketAddr>, sni: Option<ServerName>) -> Self {
    Self {
      pool: HttpPool::new(addrs, sni),
    }
  }

  pub(crate) async fn send(&self, req: Request<Body>) -> PuxResult<Response<Body>> {
    Ok(self.pool.send(req).await.unwrap())
  }
}
