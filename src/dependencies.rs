use serde::Deserialize;
use crate::dependency::Dependency;

#[derive(Debug, Deserialize)]
pub struct Dependencies {
	pub dependency: Vec<Dependency>,
}