use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::Arc;

use crate::service::Service;

pub struct Routes(HashMap<String, Vec<(Vec<String>, Arc<dyn Service + Send + Sync>)>>);

impl Routes {
  pub fn new() -> Self {
    Self(Default::default())
  }

  pub fn insert(
    &mut self,
    host: String,
    path: Vec<String>,
    service: Arc<dyn Service + Send + Sync>,
  ) {
    match self.0.entry(host) {
      Entry::Occupied(mut occupied) => {
        let paths = occupied.get_mut();
        paths.push((path, service));
        paths.sort_by(|(a, _), (b, _)| a.len().cmp(&b.len()))
      }
      Entry::Vacant(vacant) => {
        vacant.insert(vec![(path, service)]);
      }
    };
  }

  pub fn find(
    &self,
    supplied_host: &str,
    supplied_path: &[&str],
  ) -> Option<&Arc<dyn Service + Send + Sync>> {
    let paths = self.0.get(supplied_host)?;

    for (path, service) in paths {
      if starts_with(path, supplied_path) {
        return Some(service);
      }
    }

    None
  }
}

fn starts_with(base: &[String], supplied: &[&str]) -> bool {
  if base.len() > supplied.len() {
    return false;
  }

  for (i, segment) in base.iter().enumerate() {
    if supplied[i] != segment {
      return false;
    }
  }

  true
}
