mod initialize;

use std::path::Path;

use anyhow::Error;
use serde_json::Value as Json;

pub fn initialize(root_path: &Path, process_id: u32) -> Result<Json, Error> {
  crate::messages::initialize::json(root_path, process_id)
}
