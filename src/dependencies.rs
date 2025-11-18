use crate::dependency::Dependency;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Dependencies {
    #[serde(rename = "dependency")]
    #[serde(default)]
    pub dependency: Vec<Dependency>,
}
