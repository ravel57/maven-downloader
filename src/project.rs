use crate::dependencies::Dependencies;
use crate::dependency::Dependency;
use crate::properties::Properties;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename = "project")] // корневой тег <project>
pub struct Project {
    #[serde(rename = "groupId", default)]
    pub group_id: Option<String>,

    #[serde(rename = "artifactId")]
    pub artifact_id: String,

    #[serde(rename = "version", default)]
    pub version: Option<String>,

    #[serde(rename = "parent", default)]
    pub parent: Option<Dependency>,

    #[serde(rename = "dependencies", default)]
    pub dependencies: Option<Dependencies>,

    #[serde(rename = "properties", default)]
    pub properties: Option<Properties>,
}
