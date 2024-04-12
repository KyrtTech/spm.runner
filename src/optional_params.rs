use std::fs;

use serde_json;
use std::collections::HashMap;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "lowercase")] // Use lowercase for the JSON representation
enum UriType {
    Inline,
    Path,
}

// Define the structure for the payload
#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(untagged)]
enum Payload {
    Path(String),
    Json(serde_json::Value),
}

// Define the structure for each URI entry
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct UriEntry {
    #[serde(rename = "type")]
    uri_type: UriType,
    payload: Payload,
}

impl UriEntry {
    pub fn write_to_file(self: &Self, path: &str) {
        match self.uri_type {
            UriType::Inline => {
                let _ = fs::write(
                    path,
                    serde_json::to_string(&self.payload).unwrap_or_default(),
                );
            }
            UriType::Path => {
                let _ = match &self.payload {
                    Payload::Path(from) => fs::copy(from, path),
                    Payload::Json(_) => todo!(),
                };
            }
        }
    }
}

// The top-level structure represents the user provided params
pub type UserOptionalParams = HashMap<String, HashMap<String, UriEntry>>;

pub fn load_optional_params(path: &str) -> UserOptionalParams {
    if path.eq("") {
        return UserOptionalParams::default();
    };

    let file_content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => "".to_string(),
    };

    let optional_params =
        serde_json::from_str::<UserOptionalParams>(&file_content).unwrap_or_default();

    return optional_params;
}
