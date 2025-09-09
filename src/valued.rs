use serde_json::Value as JsonValue;
use valuable::{Listable, Mappable, Valuable, Value, Visit};

use crate::utils::Utils;

pub struct Valued<'a>(pub &'a JsonValue);

impl Valuable for Valued<'_> {
  fn as_value(&self) -> Value<'_> {
    match &self.0 {
      JsonValue::Null => Value::Unit,
      JsonValue::Bool(boolean) => Value::Bool(*boolean),
      JsonValue::Number(number) => {
        // NOTE: intentioanlly check i128 and u128 before f64
        if let Some(num_i128) = number.as_i128() {
          Value::I128(num_i128)
        } else if let Some(num_u128) = number.as_u128() {
          Value::U128(num_u128)
        } else if let Some(num_f64) = number.as_f64() {
          Value::F64(num_f64)
        } else {
          Value::F64(f64::NAN)
        }
      }
      JsonValue::String(string) => Value::String(string),
      JsonValue::Array(_json_values) => Value::Listable(self),
      JsonValue::Object(_json_map) => Value::Mappable(self),
    }
  }

  fn visit(&self, visit: &mut dyn Visit) {
    match self.0 {
      JsonValue::Array(json_values) => json_values
        .iter()
        .for_each(|value| visit.visit_value(value.valued().as_value())),
      JsonValue::Object(json_map) => json_map
        .iter()
        .for_each(|(key, value)| visit.visit_entry(Value::String(key), value.valued().as_value())),
      _json => visit.visit_value(self.as_value()),
    }
  }
}

impl Listable for Valued<'_> {
  fn size_hint(&self) -> (usize, Option<usize>) {
    match &self.0 {
      JsonValue::Array(json_values) => json_values.iter().size_hint(),
      _ => (0, 0.some()),
    }
  }
}

impl Mappable for Valued<'_> {
  fn size_hint(&self) -> (usize, Option<usize>) {
    match &self.0 {
      JsonValue::Object(json_map) => json_map.iter().size_hint(),
      _ => (0, 0.some()),
    }
  }
}
