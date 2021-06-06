////////////////////////////////////
// kdd - Main module file
////

mod block;
mod build;
mod builder;
mod docker;
pub mod error;
mod klog;
mod ktemplate;
mod kube;
mod loader;
mod provider;
mod realm;

use handlebars::Handlebars;
use std::{
	collections::HashMap,
	path::{Path, PathBuf},
};

use crate::utils::exec_to_stdout;

use self::{block::Block, builder::Builder, error::KddError, realm::Realm};
use indexmap::IndexMap;
use serde_json::Value;

#[derive(Debug)]
pub struct Kdd<'a> {
	hbs: Handlebars<'a>,
	vars: HashMap<String, String>,

	dir: PathBuf,
	system: String,
	block_base_dir: Option<String>,
	image_tag: Option<String>,

	realms: IndexMap<String, Realm>,
	blocks: Vec<Block>,
	builders: Vec<Builder>,
}

#[derive(Debug)]
pub struct Pod {
	pub name: String,
	pub service_name: String,
}

//// Kdev element info implementations
impl<'a> Kdd<'a> {
	/// Returns the director path of this block dir (relative to cwd)
	pub fn get_block_dir(&self, block: &Block) -> PathBuf {
		let path = match &block.dir {
			Some(path) => Path::new(path).to_path_buf(),
			None => match &self.block_base_dir {
				Some(base) => Path::new(base).join(&block.name),
				None => Path::new(&block.name).to_path_buf(),
			},
		};

		self.dir.join(path)
	}

	/// Build path from kdd path or block path if starts with ./
	pub fn get_rel_path(&self, block: &Block, path: &str) -> PathBuf {
		// if starts with "./" then relative to block dir
		if path.starts_with("./") {
			let block_dir = self.get_block_dir(&block);
			block_dir.join(&path[2..])
		} else {
			self.dir.join(path)
		}
	}

	pub fn image_tag(&self) -> String {
		match &self.image_tag {
			Some(image_tag) => image_tag.to_string(),
			None => "default".to_string(),
		}
	}

	pub fn image_name(&self, block: &Block) -> String {
		format!("{}-{}", self.system, block.name)
	}
}

//// Kdev Kubectl Queries
impl<'a> Kdd<'a> {
	fn k_list_pods(&self) -> Result<Vec<Pod>, KddError> {
		let json_pods = Self::k_get_json_items("pod")?;
		let mut pods: Vec<Pod> = Vec::new();

		for json_pod in json_pods {
			match (json_pod.pointer("/metadata/name"), json_pod.pointer("/metadata/labels/run")) {
				(Some(Value::String(pod_name)), Some(Value::String(service_name))) => {
					pods.push(Pod {
						name: pod_name.to_owned(),
						service_name: service_name.to_owned(),
					});
				}
				_ => {
					// println!("->> UNKNOWN\n{}\n\n", to_string_pretty(pod).unwrap());
				}
			}
		}
		Ok(pods)
	}

	fn k_get_json_items(entity_type: &str) -> Result<Vec<Value>, KddError> {
		let args = &["get", entity_type, "-o", "json"];
		let json = exec_to_stdout(None, "kubectl", args)?;
		let mut json = serde_json::from_str::<Value>(&json)?;

		match json["items"].take() {
			Value::Array(items) => Ok(items),
			_ => Err(KddError::KGetObjectsEmpty(entity_type.to_string())),
		}
	}
}
