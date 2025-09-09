mod initialize;

use std::path::Path;

use anyhow::Error;
use serde_json::Value as JsonValue;

use crate::utils::Utils;

pub fn initialize(root_path: &Path, process_id: u32) -> Result<JsonValue, Error> {
  let root_path = root_path.absolute()?;
  let root_uri = root_path.to_uri()?;
  let root_name = root_path.file_name_ok()?;
  let initialize_json = crate::messages::initialize::json(&root_path, &root_uri, root_name, process_id);

  initialize_json.ok()
}
