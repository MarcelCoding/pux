use std::collections::HashMap;
use std::fs::File;
use std::sync::Arc;

use tokio::signal::ctrl_c;
use tokio::{select, signal};
use tokio_rustls::rustls::client::ServerName;
use tokio_rustls::rustls::sign::CertifiedKey;
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::webpki::DnsNameRef;
use tracing::{error, info};

use crate::cert::{load_certs, load_private_key, CertStore};
use crate::config::{CertificateConfig, Config};
use crate::entrypoint::Entrypoint;
use crate::error::PuxResult;
use crate::handler::Handler;
use crate::pux::Pux;
use crate::routes::Routes;
use crate::service::proxy::ProxyService;
use crate::service::Service;
use crate::upstream::Upstream;

mod cert;
mod config;
mod entrypoint;
mod error;
mod handler;
mod pux;
mod routes;
mod service;
mod upstream;

#[tokio::main]
async fn main() -> PuxResult<()> {
  tracing_subscriber::fmt::init();

  let config_path = std::env::current_dir()?.join("config.yaml");

  let config: Config = {
    let config = File::open(&config_path).unwrap();
    serde_yaml::from_reader(config).unwrap()
  };

  let cert_store = Arc::new(build_cert_store(config.certs));

  info!("Loaded configuration at {}", config_path.display());

  let mut upstreams = HashMap::new();
  for conf in config.upstreams {
    upstreams.insert(
      conf.id,
      Arc::new(
        Upstream::new(
          conf.addrs,
          conf.sni.map(|name| ServerName::try_from(&*name).unwrap()),
        )
        .await,
      ),
    );
  }

  let mut services: HashMap<String, Arc<dyn Service + Send + Sync>> =
    HashMap::with_capacity(config.services.proxy.len());

  for config in config.services.proxy {
    services.insert(
      config.id,
      Arc::new(ProxyService::new(
        upstreams.get(&config.upstream).unwrap().clone(),
      )),
    );
  }

  let mut entrypoints = Vec::with_capacity(config.entrypoints.len());
  for cfg in config.entrypoints {
    let mut routes = Routes::new();
    for route in &config.routes {
      if route.entrypoints.contains(&cfg.id) {
        routes.insert(
          route.host.to_string(),
          route.path.clone(),
          services.get(&route.service).unwrap().clone(),
        );
      }
    }

    let handler = Arc::new(Handler::new(routes));

    let tls_config = if cfg.tls {
      let mut config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_cert_resolver(cert_store.clone());

      config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

      Some(Arc::new(config))
    } else {
      None
    };

    match Entrypoint::bind(&cfg, handler, tls_config).await {
      Ok(entrypoint) => {
        entrypoints.push(entrypoint);
        info!("Entrypoint {} bound to {}", cfg.id, cfg.addr,);
      }
      Err(err) => {
        error!(
          "Failed to bind entrypoint {} to {}: {}",
          cfg.id, cfg.addr, err
        );
      }
    };
  }

  let pux = Pux::new(entrypoints);

  select! {
    res = pux.start() => {
      if let Err(err) = res {
        error!("Server Error: {}", err);
        std::process::exit(1);
      }
    },
    _ = shutdown_signal() => {
      // info!("Shutdown signal received. Graceful shutdown will be performed...");
      // if let Err(err) = pux.shutdown() {
      //   error!("Unable to stop server: {}", err);
      // }
    }
  }

  Ok(())
}

fn build_cert_store(certs: Vec<CertificateConfig>) -> CertStore {
  let mut store = CertStore::new(
    DnsNameRef::try_from_ascii_str("m4rc3l.de")
      .unwrap()
      .to_owned(),
  );

  for conf in certs {
    let certs = load_certs(&conf.chain).unwrap();
    let key = load_private_key(&conf.key).unwrap();
    let certified = Arc::new(CertifiedKey::new(certs, key));

    for name in conf.names {
      let name = DnsNameRef::try_from_ascii_str(&name).unwrap().to_owned();
      store.insert(name, certified.clone());
    }
  }

  store
}

async fn shutdown_signal() {
  let ctrl_c = async { ctrl_c().await.expect("failed to install Ctrl+C handler") };

  #[cfg(unix)]
  {
    let terminate = async {
      signal::unix::signal(signal::unix::SignalKind::terminate())
        .expect("failed to install signal handler")
        .recv()
        .await;
    };

    select! {
      _ = ctrl_c => {},
      _ = terminate => {},
    }
  }

  #[cfg(not(unix))]
  ctrl_c.await;
}
