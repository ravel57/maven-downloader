use crate::dependencies::Dependencies;
use serde::Deserialize;
use crate::dependency::Dependency;
use crate::properties::Properties;

#[derive(Debug, Deserialize)]
pub struct Project {
	#[serde(rename = "groupId")]
	pub group_id: Option<String>,
	#[serde(rename = "artifactId")]
	pub artifact_id: String,
	#[serde(rename = "version")]
	pub version: Option<String>,
	#[serde(rename = "parent")]
	pub parent: Option<Dependency>,
	#[serde(rename = "dependencies")]
	pub dependencies: Option<Dependencies>,
	#[serde(rename = "properties")]
	pub properties: Option<Properties>,
}