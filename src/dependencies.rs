use crate::dependency::Dependency;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Dependencies {
    #[serde(rename = "dependency")]
    #[serde(default)]
    pub dependency: Vec<Dependency>,
}
