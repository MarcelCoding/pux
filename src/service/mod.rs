use async_trait::async_trait;
use hyper::{Body, Request, Response};

use crate::PuxResult;

pub mod proxy;

#[async_trait]
pub trait Service {
  async fn handle(&self, req: Request<Body>) -> PuxResult<Response<Body>>;
}
