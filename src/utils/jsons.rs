use serde_json::Value;

pub fn as_string(value: &Value, pointer: &str) -> Option<String> {
	value.pointer(pointer).and_then(|v| v.as_str().map(|v| v.to_string()))
}
