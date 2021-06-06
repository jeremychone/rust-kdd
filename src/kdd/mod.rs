////////////////////////////////////
// kdd - Main module file
////

mod build;
mod docker;
pub mod error;
mod exec;
mod klog;
mod ktemplate;
mod kube;
mod loader;
mod provider;
mod realm;

use fs::read_dir;
use handlebars::Handlebars;
use std::{
	collections::{HashMap, HashSet},
	fs,
	path::{Path, PathBuf},
};
use yaml_rust::Yaml;

use self::{
	error::KddError,
	exec::Exec,
	provider::{AwsProvider, DesktopProvider, Provider, RealmProvider},
};
use indexmap::IndexMap;

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

// region:    Realm
#[derive(Debug)]
pub struct Realm {
	pub name: String,
	pub confirm_delete: bool,
	pub vars: HashMap<String, String>,
	pub provider: RealmProvider,
	yaml_dirs: Vec<PathBuf>,
	pub context: Option<String>,
	pub registry: Option<String>,
	pub profile: Option<String>,
	pub project: Option<String>,
	pub default_configurations: Option<Vec<String>>,
}

impl Realm {
	pub fn provider_from_ctx(ctx: &str) -> Result<RealmProvider, KddError> {
		if ctx.contains("docker-desktop") {
			Ok(RealmProvider::Desktop(DesktopProvider))
		} else if ctx.starts_with("arn:aws") {
			Ok(RealmProvider::Aws(AwsProvider))
		} else {
			Err(KddError::ContextNotSupported(ctx.to_string()))
		}
	}

	pub fn provider(&self) -> &dyn Provider {
		match &self.provider {
			RealmProvider::Aws(p) => p as &dyn Provider,
			RealmProvider::Desktop(p) => p as &dyn Provider,
		}
	}

	pub fn profile(&self) -> String {
		self.profile.as_deref().unwrap_or("default").to_string()
	}

	pub fn k8s_files(&self, names: Option<&[&str]>) -> Vec<PathBuf> {
		let mut yaml_paths: Vec<PathBuf> = Vec::new();

		// if we have a names above
		if let Some(names) = names {
			for name in names {
				let mut yaml_dirs = self.yaml_dirs.iter();
				loop {
					match yaml_dirs.next() {
						Some(dir_path) => {
							let yaml_path = dir_path.join(format!("{}.yaml", name));
							if yaml_path.is_file() {
								yaml_paths.push(yaml_path);
								break;
							}
						}
						None => break,
					}
				}
			}
		}
		// otherwise, get all of the file (first  file_stem wins)
		else {
			let mut stems_set: HashSet<String> = HashSet::new();

			for yaml_dir in &self.yaml_dirs {
				if let Ok(paths) = read_dir(yaml_dir) {
					for path in paths {
						if let Ok(path) = path {
							let path = path.path();
							if let (Some(stem), Some(ext)) = (path.file_stem().map(|v| v.to_str()).flatten(), path.extension().map(|v| v.to_str()).flatten()) {
								if path.is_file() && ext.to_lowercase() == "yaml" && !stems_set.contains(stem) {
									stems_set.insert(stem.to_string());
									yaml_paths.push(path);
								}
							}
						}
					}
				}
			}
		}

		// Note: Should be ok to be lossy here
		yaml_paths.sort_by_key(|v| v.file_stem().unwrap().to_string_lossy().to_string());

		yaml_paths
	}

	pub fn k8s_out_dir(&self) -> PathBuf {
		// Note: There is always at least one yaml_dir, per parse_realm logic
		let yaml_dir = &self.yaml_dirs[0];
		yaml_dir.join(".out/").join(&self.name)
	}
}
// endregion: Realm

// region:    Block
#[derive(Debug, Default)]
pub struct Block {
	pub name: String,
	pub dir: Option<String>,
	pub dependencies: Option<Vec<String>>,
	pub map: Option<Yaml>,
}

#[derive(Debug)]
pub struct Builder {
	name: String,
	when_file: Option<String>,
	exec: Exec,
}
// endregion: Block

// region:    Kdev Impls

// Kdev element info implementations
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
// endregion: Kdev Impls
