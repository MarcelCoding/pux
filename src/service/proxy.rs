use std::sync::Arc;

use async_trait::async_trait;
use hyper::header::CONNECTION;
use hyper::http::HeaderValue;
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
  async fn handle(&self, mut req: Request<Body>) -> PuxResult<Response<Body>> {
    req
      .headers_mut()
      .insert(CONNECTION, HeaderValue::from_static("keep-alive"));
    self.upstream.send(req).await
  }
}
