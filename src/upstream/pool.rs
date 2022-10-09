use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use hyper::{Body, Request, Response};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tokio_rustls::rustls::ServerName;
use tracing::error;

use crate::upstream::conn::HttpConnection;
use crate::upstream::error::Error;

pub(crate) struct HttpPool {
  internal: Arc<Mutex<Internal>>,
  sni: Option<ServerName>,
}

struct Internal {
  // todo: use concurrent hash map: https://docs.rs/flurry
  conns: HashMap<SocketAddr, Vec<Instant>>,
  idle: Vec<Entry>,
  force_use: Duration,
}

struct Entry {
  idle_since: Instant,
  id: Instant,
  conn: HttpConnection,
}

enum SelectResult {
  Conn(HttpConnection),
  Addr(SocketAddr),
}

impl HttpPool {
  pub(crate) fn new(addrs: Vec<SocketAddr>, sni: Option<ServerName>) -> Self {
    let mut conns = HashMap::with_capacity(addrs.len());

    for addr in addrs {
      conns.insert(addr, vec![]);
    }

    let internal = Arc::new(Mutex::new(Internal {
      conns,
      idle: vec![],
      force_use: Duration::from_millis(10),
    }));

    let internal_clone = internal.clone();
    tokio::spawn(async move {
      loop {
        sleep(Duration::from_secs(2)).await;
        let mut internal = internal_clone.lock().await;
        internal.clean()
      }
    });

    Self { internal, sni }
  }

  pub(crate) async fn send(&self, req: Request<Body>) -> Result<Response<Body>, Error> {
    let (id, result) = {
      let mut internal = self.internal.lock().await;
      match internal.select() {
        None => {
          let (id, addr) = internal.select_addr();
          (id, SelectResult::Addr(addr))
        }
        Some((id, conn)) => (id, SelectResult::Conn(conn)),
      }
    };

    let mut conn = match result {
      SelectResult::Conn(conn) => conn,
      SelectResult::Addr(addr) => match HttpConnection::open(&addr, &self.sni).await {
        Ok(conn) => conn,
        Err(err) => {
          self.internal.lock().await.remove_conn(&id);
          return Err(err);
        }
      },
    };

    let resp = conn.send(req).await;

    let internal_clone = self.internal.clone();
    tokio::spawn(async move {
      if let Err(err) = conn.ready().await {
        internal_clone.lock().await.remove_conn(&id);
        error!("Connection closed: {}", err);
      } else {
        internal_clone.lock().await.push(id, conn);
      }
    });

    resp
  }
}

impl Internal {
  fn select(&mut self) -> Option<(Instant, HttpConnection)> {
    let mut candidate = None;

    let force_use = Instant::now() - self.force_use;

    for (i, entry) in self.idle.iter().rev().enumerate() {
      if force_use >= entry.idle_since {
        candidate = Some((i, &entry.idle_since));
        break;
      }

      match candidate {
        None => candidate = Some((i, &entry.idle_since)),
        Some((_, best_idle_since)) => {
          if best_idle_since > &entry.idle_since {
            candidate = Some((i, &entry.idle_since))
          }
        }
      }
    }

    // drop reference &Instant from Option<(usize, &Instant)> in candidate
    let candidate = candidate.map(|(i, _)| i);

    candidate.map(|i| {
      let entry = self.idle.remove(i);
      (entry.id, entry.conn)
    })
  }

  fn select_addr(&mut self) -> (Instant, SocketAddr) {
    let mut candidate = None;

    for (addr, conns) in &self.conns {
      match candidate {
        None => candidate = Some((addr, conns.len())),
        Some((_, low_conns)) => {
          if conns.len() < low_conns {
            candidate = Some((addr, conns.len()))
          }
        }
      }
    }

    let candidate = *candidate.expect("No addresses for upstream provided").0;
    let id = Instant::now();

    self.conns.get_mut(&candidate).unwrap().push(id);

    (id, candidate)
  }

  fn push(&mut self, id: Instant, conn: HttpConnection) {
    self.idle.push(Entry {
      idle_since: Instant::now(),
      id,
      conn,
    });
  }

  fn remove_conn(&mut self, id: &Instant) {
    for conns in self.conns.values_mut() {
      conns.retain(|c_id| c_id != id)
    }
  }

  fn clean(&mut self) {
    let mut to_delete = Vec::new();

    let idle_since_to_close = Instant::now() - Duration::from_secs(10);

    for (i, entry) in self.idle.iter().enumerate() {
      if entry.idle_since < idle_since_to_close {
        to_delete.push(i)
      }
    }

    for (i, index) in to_delete.iter().enumerate() {
      let entry = self.idle.remove(index - i);
      self.remove_conn(&entry.id);
    }
  }
}
