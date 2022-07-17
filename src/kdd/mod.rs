////////////////////////////////////
// kdd - Main module file
////

mod block;
mod build;
mod builder;
mod docker;
pub mod error;
mod kctl;
mod kevents;
mod kexec;
mod klog;
mod ktemplate;
mod loader;
mod provider;
mod realm;
pub mod version;

use handlebars::Handlebars;
use std::collections::HashSet;
use std::{
	collections::HashMap,
	path::{Path, PathBuf},
};

use crate::utils::exec_to_stdout;

use self::{block::Block, builder::Builder, error::KddError, realm::Realm, version::Version};
use indexmap::IndexMap;
use serde_json::Value;

#[derive(Debug)]
pub struct KddConfig {
	// hbs: Handlebars,
	vars: HashMap<String, String>,

	dir: PathBuf,
	system: String,
	block_base_dir: Option<String>,
	image_tag: Option<String>,

	realms: IndexMap<String, Realm>,
	blocks: Vec<Block>,
	builders: Vec<Builder>,
	versions: Vec<Version>,
}

#[derive(Debug)]
pub struct Kdd {
	// hbs: Handlebars,
	vars: HashMap<String, String>,

	dir: PathBuf,
	system: String,
	block_base_dir: Option<String>,
	image_tag: Option<String>,

	realms: IndexMap<String, Realm>,
	blocks: Vec<Block>,
	builders: Vec<Builder>,
	versions: Vec<Version>,

	pods_provider: PodsProvider,
}

#[derive(Debug)]
pub struct Pod {
	pub name: String,
	pub service_name: String,
}

impl From<KddConfig> for Kdd {
	fn from(config: KddConfig) -> Self {
		let pods_provider = PodsProvider {
			system: config.system.clone(),
		};

		Kdd {
			vars: config.vars,

			dir: config.dir,
			system: config.system,
			block_base_dir: config.block_base_dir,
			image_tag: config.image_tag,

			realms: config.realms,
			blocks: config.blocks,
			builders: config.builders,
			versions: config.versions,

			pods_provider,
		}
	}
}

/// Kdd basic methods
impl Kdd {
	/// Returns the directory path of this block dir (relative to cwd)
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

// region:    --- PodsProvider

impl Kdd {
	pub fn get_pods_by_service_names(&self, service_names: &Option<Vec<String>>) -> Result<Vec<Pod>, KddError> {
		self.pods_provider.get_pods_by_service_names(service_names)
	}

	pub fn get_pods_provider(&self) -> PodsProvider {
		self.pods_provider.clone()
	}
}

#[derive(Clone, Debug)]
pub struct PodsProvider {
	system: String,
}

impl PodsProvider {
	pub fn get_pods_by_service_names(&self, service_names: &Option<Vec<String>>) -> Result<Vec<Pod>, KddError> {
		// get all the pods
		let mut pods = Kdd::k_list_pods()?;

		// filter the names
		if let Some(names) = service_names {
			let names_set: HashSet<String> = names.iter().map(|v| format!("{}-{}", self.system, v)).collect();
			// TODO - The contains() is too broad here. If we have `cstar-cmd` pod `cstar-cmd-rs` will amach as well.
			//        Not a critical issue, but should be fixed
			pods = pods.into_iter().filter(|pod| names_set.contains(&pod.service_name)).collect();
		}

		Ok(pods)
	}
}
// endregion: --- PodsProvider

/// Static Kubectl Queries
impl Kdd {
	fn k_list_pods() -> Result<Vec<Pod>, KddError> {
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
				_ => {}
			}
		}
		Ok(pods)
	}

	fn k_get_json_items(entity_type: &str) -> Result<Vec<Value>, KddError> {
		let args = &["get", entity_type, "-o", "json"];
		let json = exec_to_stdout(None, "kubectl", args, false)?;
		let mut json = serde_json::from_str::<Value>(&json)?;

		match json["items"].take() {
			Value::Array(items) => Ok(items),
			_ => Err(KddError::KGetObjectsEmpty(entity_type.to_string())),
		}
	}
}
