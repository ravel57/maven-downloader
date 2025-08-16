use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Dependency {
	#[serde(rename = "groupId")]
	pub group_id: String,
	#[serde(rename = "artifactId")]
	pub artifact_id: String,
	#[serde(rename = "version")]
	pub version: Option<String>,
}