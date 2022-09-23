use std::sync::Arc;
use async_trait::async_trait;
use hyper::{Body, Request, Response};

use crate::service::Service;
use crate::upstream::Upstream;
use crate::PuxResult;

pub struct ProxyService {
  upstream: Arc<Upstream>,
}

impl ProxyService {
  pub fn new(upstream: Arc<Upstream>) -> Self {
    Self { upstream }
  }
}

#[async_trait]
impl Service for ProxyService {
  async fn handle(&self, req: Request<Body>) -> PuxResult<Response<Body>> {
    self.upstream.send(req).await
  }
}
