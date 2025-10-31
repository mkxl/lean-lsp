use serde_json::{Map, Value as Json};
use valuable::{Listable, Mappable, Valuable, Value as ValuableValue, Visit};

pub trait ToValue {
  fn to_value(&self) -> &'_ dyn Valuable;
}

impl ToValue for Json {
  fn to_value(&self) -> &'_ dyn Valuable {
    JsonValuable::new(self) as _
  }
}

#[repr(transparent)]
struct JsonValuable(Json);

impl JsonValuable {
  fn new(json: &Json) -> &JsonValuable {
    let json_ptr = std::ptr::from_ref(json).cast::<Self>();
    // Safety: `&Json` and `&JsonValuable` have the same layout.
    let json_ref = unsafe { json_ptr.as_ref() };

    json_ref.unwrap()
  }
}

#[repr(transparent)]
struct JsonValuableMap(Map<String, Json>);

impl JsonValuableMap {
  fn new(map: &Map<String, Json>) -> &JsonValuableMap {
    let map_ptr = std::ptr::from_ref(map).cast::<Self>();
    // Safety: `&Map<String, Json>` and `&JsonValuableMap` have the same layout.
    let map_ref = unsafe { map_ptr.as_ref() };

    map_ref.unwrap()
  }
}

#[repr(transparent)]
struct JsonValuableArray(Vec<Json>);

impl JsonValuableArray {
  fn new(array: &Vec<Json>) -> &JsonValuableArray {
    let array_ptr = std::ptr::from_ref(array).cast::<Self>();
    // Safety: `&Vec<Json>` and `&JsonValuableArray` have the same layout.
    let array_ref = unsafe { array_ptr.as_ref() };

    array_ref.unwrap()
  }
}

impl Valuable for JsonValuable {
  fn as_value(&self) -> ValuableValue<'_> {
    match &self.0 {
      Json::Null => ValuableValue::Unit,
      Json::Bool(boolean) => boolean.as_value(),
      Json::Number(number) => {
        // NOTE: intentionally check i128 and u128 before f64
        if let Some(number) = number.as_i128() {
          ValuableValue::I128(number)
        } else if let Some(number) = number.as_u128() {
          ValuableValue::U128(number)
        } else if let Some(number) = number.as_f64() {
          ValuableValue::F64(number)
        } else {
          f64::NAN.as_value()
        }
      }
      Json::String(string) => string.as_value(),
      Json::Array(array) => JsonValuableArray::new(array).as_value(),
      Json::Object(map) => JsonValuableMap::new(map).as_value(),
    }
  }

  fn visit(&self, visit: &mut dyn Visit) {
    self.as_value().visit(visit);
  }
}

impl Valuable for JsonValuableMap {
  fn as_value(&self) -> ValuableValue<'_> {
    ValuableValue::Mappable(self)
  }

  fn visit(&self, visit: &mut dyn Visit) {
    for (k, v) in &self.0 {
      visit.visit_entry(k.as_value(), JsonValuable::new(v).as_value());
    }
  }
}

impl Mappable for JsonValuableMap {
  fn size_hint(&self) -> (usize, Option<usize>) {
    self.0.iter().size_hint()
  }
}

impl Valuable for JsonValuableArray {
  fn as_value(&self) -> ValuableValue<'_> {
    ValuableValue::Listable(self)
  }

  fn visit(&self, visit: &mut dyn Visit) {
    for v in &self.0 {
      visit.visit_value(JsonValuable::new(v).as_value());
    }
  }
}

impl Listable for JsonValuableArray {
  fn size_hint(&self) -> (usize, Option<usize>) {
    self.0.iter().size_hint()
  }
}
