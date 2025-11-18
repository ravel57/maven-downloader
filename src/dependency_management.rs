use crate::dependencies::Dependencies;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct DependencyManagement {
    #[serde(rename = "dependencies", default)]
    pub dependencies: Option<Dependencies>,
}
