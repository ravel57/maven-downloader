use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Dependency {
    #[serde(rename = "groupId")]
    pub group_id: String,
    #[serde(rename = "artifactId")]
    pub artifact_id: String,
    #[serde(rename = "version", default)]
    pub version: Option<TextOrNode>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum TextOrNode {
    Text(String),
    Node {
        #[serde(rename = "$text")]
        text: Option<String>,
    },
}

impl TextOrNode {
    pub fn into_string(self) -> Option<String> {
        match self {
            TextOrNode::Text(s) => Some(s),
            TextOrNode::Node { text } => text,
        }
    }
}
