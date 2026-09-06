use language_core::{ImageRef, Value};
use serde::de::{self, Deserializer, MapAccess, Visitor};
use std::collections::{HashMap, HashSet};
use storage::{AppFs, inspect_image};

use crate::server_errors::UploadRuntimeError;

pub(super) async fn build_upload_runtime_value(
    fs: &AppFs,
    upload: &language_core::UploadField,
    destination: &str,
    info: storage::UploadResult,
    max_image_pixels: u64,
) -> Result<Value, UploadRuntimeError> {
    if upload.image {
        let bytes = fs.read(destination).await?;
        let image = inspect_image(&bytes, max_image_pixels)?;
        let reference = ImageRef::new(
            destination.to_string(),
            image.content_type.to_string(),
            image.width,
            image.height,
            info.bytes_written,
        )
        .ok_or(UploadRuntimeError::InvalidImageReference)?;
        Ok(Value::Image(reference))
    } else {
        let mut upload_value = HashMap::new();
        upload_value.insert("path".into(), Value::String(destination.to_string()));
        upload_value.insert(
            "filename".into(),
            Value::String(info.original_filename.unwrap_or_default()),
        );
        upload_value.insert(
            "contentType".into(),
            Value::String(info.content_type.unwrap_or_default()),
        );
        upload_value.insert(
            "bytes".into(),
            Value::Int(i64::try_from(info.bytes_written).unwrap_or(i64::MAX)),
        );
        Ok(Value::Record(upload_value))
    }
}

pub(super) fn media_type_is(value: &str, wanted: &str) -> bool {
    value
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .eq_ignore_ascii_case(wanted)
}

struct StrictJsonObjectVisitor {
    max_fields: usize,
    max_field_bytes: usize,
}

impl<'de> Visitor<'de> for StrictJsonObjectVisitor {
    type Value = Vec<(String, String)>;

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("a JSON object with scalar values")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if out.len() >= self.max_fields {
                return Err(de::Error::custom("too many JSON fields"));
            }
            if key.is_empty()
                || key.len() > self.max_field_bytes
                || key.bytes().any(|b| b < 0x20 || b == 0x7f)
                || !seen.insert(key.clone())
            {
                return Err(de::Error::custom("invalid or duplicate JSON field"));
            }
            let value = map.next_value::<serde_json::Value>()?;
            let raw = match value {
                serde_json::Value::String(v)
                    if v.len() <= self.max_field_bytes && !v.bytes().any(|b| b == 0) =>
                {
                    v
                }
                serde_json::Value::Number(v) if v.as_i64().is_some() => v.to_string(),
                serde_json::Value::Bool(v) => {
                    if v {
                        "true".into()
                    } else {
                        "false".into()
                    }
                }
                _ => return Err(de::Error::custom("JSON field must be String, Int, or Bool")),
            };
            if raw.len() > self.max_field_bytes {
                return Err(de::Error::custom("JSON field value too large"));
            }
            out.push((key, raw));
        }
        Ok(out)
    }
}

pub(super) fn decode_json_object_limited(
    body: &[u8],
    max_fields: usize,
    max_field_bytes: usize,
) -> Result<Vec<(String, String)>, ()> {
    let mut de = serde_json::Deserializer::from_slice(body);
    let value = de
        .deserialize_map(StrictJsonObjectVisitor {
            max_fields,
            max_field_bytes,
        })
        .map_err(|_| ())?;
    de.end().map_err(|_| ())?;
    Ok(value)
}
