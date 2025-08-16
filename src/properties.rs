use std::collections::HashMap;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Properties {
	#[serde(flatten)]
	pub values: HashMap<String, String>,
}