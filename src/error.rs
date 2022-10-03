use std::fmt::{Debug, Display, Formatter};
use std::io::Error;

use hyper::{http, StatusCode};

pub(crate) type PuxResult<T> = Result<T, PuxError>;

pub(crate) enum PuxError {
  Io(Error),
  Http(http::Error),
  Hyper(hyper::Error),
  Status(StatusCode),
}

impl Display for PuxError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Io(err) => write!(f, "IO Error: {}", err),
      Self::Http(err) => write!(f, "Http Error: {}", err),
      Self::Hyper(err) => write!(f, "Hyper Error: {}", err),
      Self::Status(code) => write!(
        f,
        "Status Code: {} {}",
        code.as_u16(),
        code.canonical_reason().unwrap_or("")
      ),
    }
  }
}

impl Debug for PuxError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    Display::fmt(self, f)
  }
}

impl From<Error> for PuxError {
  fn from(err: Error) -> Self {
    Self::Io(err)
  }
}

impl From<http::Error> for PuxError {
  fn from(err: http::Error) -> Self {
    Self::Http(err)
  }
}

impl From<hyper::Error> for PuxError {
  fn from(err: hyper::Error) -> Self {
    Self::Hyper(err)
  }
}

impl From<StatusCode> for PuxError {
  fn from(code: StatusCode) -> Self {
    Self::Status(code)
  }
}
