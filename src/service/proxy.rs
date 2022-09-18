use async_trait::async_trait;
use hyper::{Body, Request, Response, StatusCode};

use crate::service::Service;
use crate::upstream::Upstream;
use crate::PuxResult;

pub struct ProxyService {
  upstream: Upstream,
}

impl ProxyService {
  pub fn new(upstream: Upstream) -> Self {
    Self { upstream }
  }
}

#[async_trait]
impl Service for ProxyService {
  async fn handle(&self, req: Request<Body>) -> PuxResult<Response<Body>> {
    Err(StatusCode::NOT_IMPLEMENTED.into())
  }
}
