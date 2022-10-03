use async_trait::async_trait;
use hyper::{Body, Request, Response};

use crate::PuxResult;

pub(crate) mod proxy;

#[async_trait]
pub(crate) trait Service {
  async fn handle(&self, req: Request<Body>) -> PuxResult<Response<Body>>;
}
