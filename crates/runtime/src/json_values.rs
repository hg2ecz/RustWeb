use language_core::{AppError, Value};

pub(crate) fn serialize_json_value(value: &Value) -> Result<String, AppError> {
    fn convert(value: &Value) -> serde_json::Value {
        match value {
            Value::String(v) => serde_json::Value::String(v.clone()),
            Value::Email(v) => serde_json::Value::String(v.clone()),
            Value::Url(v) => serde_json::Value::String(v.clone()),
            Value::Int(v) => serde_json::Value::Number((*v).into()),
            Value::F32(v) => serde_json::json!(v.get()),
            Value::F32Array(items) => serde_json::Value::Array(items.iter().map(|v| serde_json::json!(v.get())).collect()),
            Value::StringList(items) => serde_json::Value::Array(items.iter().cloned().map(serde_json::Value::String).collect()),
            Value::StringDict(items) => serde_json::Value::Object(items.iter().map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone()))).collect()),
            Value::Bool(v) => serde_json::Value::Bool(*v),
            Value::Date(v) => serde_json::Value::String(v.format("%Y-%m-%d").to_string()),
            Value::DateTime(v) => serde_json::Value::String(v.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true)),
            Value::Uuid(v) => serde_json::Value::String(v.hyphenated().to_string()),
            Value::Decimal(v) => serde_json::Value::String(v.normalize().to_string()),
            Value::Image(v) => serde_json::Value::String(v.canonical()),
            Value::Enum { variant, .. } => serde_json::Value::String(variant.clone()),
            Value::Null => serde_json::Value::Null,
            Value::Record(fields) => {
                let mut map = serde_json::Map::new();
                let mut keys: Vec<_> = fields.keys().collect();
                keys.sort();
                for key in keys {
                    map.insert((*key).clone(), convert(fields.get(key).expect("record key exists")));
                }
                serde_json::Value::Object(map)
            }
            Value::List(values) => serde_json::Value::Array(values.iter().map(convert).collect()),
        }
    }
    serde_json::to_string(&convert(value)).map_err(|_| AppError::Internal)
}
