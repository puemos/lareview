use serde_json::Value;

pub fn parse_json(data: &str) -> Result<Value, String> {
    serde_json::from_str(data).map_err(|e| e.to_string())
}
