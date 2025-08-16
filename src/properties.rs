use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone)]
pub struct Properties {
    #[serde(flatten)]
    pub values: HashMap<String, PropValue>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum PropValue {
    Text(String),
    Node {
        #[serde(rename = "$text")]
        text: Option<String>,
    },
}

impl PropValue {
    pub fn into_string(self) -> Option<String> {
        match self {
            PropValue::Text(s) => Some(s),
            PropValue::Node { text } => text,
        }
    }
}
