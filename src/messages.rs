mod initialize;

use crate::utils::Utils;
use anyhow::Error;
use serde_json::Value as Json;
use std::path::Path;

pub fn initialize(root_path: &Path, process_id: u64) -> Result<Json, Error> {
  let root_path = root_path.absolute()?;
  let root_uri = root_path.to_uri()?;
  let root_name = root_path.file_name_ok()?;

  let initialize_json = initialize::initialize_json(&root_path, &root_uri, &root_name, process_id);

  initialize_json.ok()
}
