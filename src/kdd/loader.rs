////////////////////////////////////
// kdd::loader - Responsible to load and instantiate a kdd
////

use std::{collections::HashMap, fs::read_to_string, path::PathBuf};

use handlebars::Handlebars;
use indexmap::IndexMap;
use regex::Regex;
use yaml_rust::{Yaml, YamlLoader};

use crate::{
	utils::path_to_string,
	yutils::{as_bool, as_string, as_strings, as_yaml_map, merge_yaml, remove_keys},
};

use super::{error::KddError, exec::Exec, Block, Builder, Kdd, Realm};
use serde_json::Value;

const KDD_KEY_SYSTEM: &str = "system";
const KDD_KEY_BLOCK_DIR: &str = "block_base_dir";
const KDD_KEY_IMAGE_TAG: &str = "image_tag";

const REALM_KEY_YAML_DIR: &str = "yaml_dir";
const REALM_KEY_CONTEXT: &str = "context";
const REALM_KEY_CONFIRM_DELETE: &str = "confirm_delete";
const REALM_KEY_PROJECT: &str = "project"; // for GKE
const REALM_KEY_REGISTRY: &str = "registry"; // must on AWS (inferred for gke and docker-dekstop)
const REALM_KEY_PROFILE: &str = "profile"; // for AWS
const REALM_KEY_CONFIGURATIONS: &str = "default_configurations"; // for AWS

const BLOCK_KEY_NAME: &str = "name";
const BLOCK_KEY_DIR: &str = "dir";
const BLOCK_KEY_DEP: &str = "dependencies";
const BLOCK_KEYS: &[&str] = &[BLOCK_KEY_NAME, BLOCK_KEY_DIR, BLOCK_KEY_DEP];

// Kdev Builder
impl<'a> Kdd<'a> {
	pub fn from_dir(dir: PathBuf) -> Result<Kdd<'a>, KddError> {
		//// build the template engine
		let hbs: Handlebars = Handlebars::new();

		//// load the root yaml
		let kdd_path = dir.join("kdd.yaml");
		if !kdd_path.is_file() {
			return Err(KddError::NoKdevFileFound(dir.to_string_lossy().to_string()));
		}
		let kdd_content = read_to_string(kdd_path)?;
		let rx = Regex::new(r"(?m)^---.*\W").expect("works once, works all the time");
		let splits: Vec<_> = rx.split(&kdd_content).collect();
		let (kdd_yaml, mut vars) = match splits.len() {
			// if only one template, then, just the core kdd template and empty vars
			1 => (YamlLoader::load_from_str(splits[0])?, HashMap::new()),
			// if two yaml document, first ones is the vars and second is the kdd template
			2 => {
				let vars = load_vars(&dir, YamlLoader::load_from_str(splits[0])?);
				let rendered_yaml = match hbs.render_template(splits[1], &vars) {
					Ok(r) => r,
					Err(e) => return Err(KddError::KdevFailToParseInvalid(e.to_string())),
				};
				let kdd_yaml = YamlLoader::load_from_str(&rendered_yaml)?;
				(kdd_yaml, vars)
			}
			// otherwise, fail for now
			_ => {
				return Err(KddError::KdevYamlInvalid);
			}
		};

		let kdd_yaml = &kdd_yaml[0];

		//// load the base properties
		let system = as_string(kdd_yaml, KDD_KEY_SYSTEM).ok_or(KddError::NoSystem)?;

		//// read the blocks
		let blocks = parse_blocks(&kdd_yaml["blocks"]);

		//// read the realms
		let realms = parse_realms(&dir, &kdd_yaml["realms"]);

		//// read the builder
		let builders = parse_builders(&kdd_yaml["builders"]);

		vars.insert("dir".to_owned(), path_to_string(&dir)?);
		vars.insert("dir_abs".to_owned(), path_to_string(&dir.canonicalize()?)?);
		vars.insert("system".to_owned(), system.to_string());

		// add all of the root variables as vars
		if let Some(map) = kdd_yaml.as_hash() {
			for (name, val) in map.iter() {
				if let (Some(name), Some(val)) = (name.as_str(), val.as_str()) {
					vars.insert(name.to_owned(), val.to_owned());
				}
			}
		}

		let kdd = Kdd {
			hbs,
			vars,

			dir,
			system,
			block_base_dir: as_string(kdd_yaml, KDD_KEY_BLOCK_DIR),
			image_tag: as_string(kdd_yaml, KDD_KEY_IMAGE_TAG),
			blocks,
			realms,
			builders,
		};

		Ok(kdd)
	}
}

// region:    Load Vars
enum VarsSource {
	Json(PathBuf),
	NotSupported(PathBuf),
}

impl VarsSource {
	fn from_path(path: PathBuf) -> VarsSource {
		if let Some(Some(ext)) = path.extension().map(|v| v.to_str().map(|v| v.to_lowercase())) {
			match ext.as_str() {
				"json" => VarsSource::Json(path),
				_ => VarsSource::NotSupported(path),
			}
		} else {
			VarsSource::NotSupported(path)
		}
	}
}
fn load_vars(dir: &PathBuf, yamls: Vec<Yaml>) -> HashMap<String, String> {
	let mut vars = HashMap::new();

	for yaml in yamls {
		if let Some(vars_yaml) = yaml["vars"].as_vec() {
			for var_yaml in vars_yaml {
				if let (Some(file), Some(extract)) = (var_yaml["from_file"].as_str(), var_yaml["extract"].as_vec()) {
					match VarsSource::from_path(dir.join(file)) {
						VarsSource::Json(path) => match read_to_string(&path) {
							Ok(content) => match serde_json::from_str::<Value>(&content) {
								Ok(src_json) => {
									for extract_item in extract {
										if let Some(var_path) = extract_item.as_str() {
											if let Some(value) = src_json[var_path].as_str() {
												vars.insert(var_path.to_owned(), value.to_owned());
											}
										}
									}
								}
								Err(ex) => {
									println!("KDD WARNING - Invalid json for {} ex: {} - SKIP", path.to_string_lossy(), ex);
								}
							},
							Err(ex) => {
								println!("KDD WARNING - Cannot read from {} because {} - SKIP", path.to_string_lossy(), ex);
							}
						},
						VarsSource::NotSupported(path) => {
							println!("KDD WARNING - file {} not supported as a variable source. - SKIP", path.to_string_lossy());
						}
					}
				} else {
					println!(
						"KDD WARNING - vars items must have from_file and extract properties. But has {:?} - SKIP",
						var_yaml
					)
				}
			}
		}
	}
	vars
}
// endregion: Load Vars

// region:    Realms Parser
fn parse_realms(kdd_dir: &PathBuf, y_realms: &Yaml) -> IndexMap<String, Realm> {
	match y_realms.as_hash() {
		None => IndexMap::new(),
		Some(y_realms) => {
			let base = y_realms.get(&Yaml::String("_base_".to_string()));

			let mut realms: IndexMap<String, Realm> = IndexMap::new();
			for y_realm in y_realms.into_iter() {
				let (name, data) = y_realm;
				if let Some(name) = name.as_str() {
					// if name is _base_ then not a realm, continue
					if name == "_base_" {
						continue;
					}

					//// merge the data from _base_ and this realm
					let yaml_data = if let Some(base) = base { Some(merge_yaml(base, data)) } else { None };
					let yaml_data = match &yaml_data {
						Some(data) => data,
						None => &data,
					};

					match parse_realm(kdd_dir, name, yaml_data) {
						Ok(realm) => {
							realms.insert(name.to_string(), realm);
						}
						Err(ex) => println!("Faill to prase realm {}. Cause: {}", name, ex),
					}
				}
			}
			realms
		}
	}
}

fn parse_realm(kdd_dir: &PathBuf, name: &str, yaml: &Yaml) -> Result<Realm, KddError> {
	let ctx = as_string(yaml, REALM_KEY_CONTEXT).unwrap_or("docker-desktop".to_string());
	let provider = Realm::provider_from_ctx(&ctx)?;

	// NOTE: Must have at least one yaml_dir
	let yaml_dir = as_string(yaml, REALM_KEY_YAML_DIR).unwrap_or_else(|| format!("k8s/{}", name));
	let yaml_dirs = vec![kdd_dir.join(yaml_dir)];

	let mut vars: HashMap<String, String> = HashMap::new();
	// add all of the root variables as vars
	if let Some(map) = yaml.as_hash() {
		for (name, val) in map.iter() {
			if let (Some(name), Some(val)) = (name.as_str(), val.as_str()) {
				vars.insert(name.to_owned(), val.to_owned());
			}
		}
	}

	let confirm_delete = as_bool(yaml, REALM_KEY_CONFIRM_DELETE).unwrap_or(true);

	Ok(Realm {
		name: name.to_string(),
		confirm_delete,
		vars,
		provider: provider,
		yaml_dirs,
		context: as_string(yaml, REALM_KEY_CONTEXT),
		project: as_string(yaml, REALM_KEY_PROJECT),
		registry: as_string(yaml, REALM_KEY_REGISTRY),
		profile: as_string(yaml, REALM_KEY_PROFILE),
		default_configurations: as_strings(yaml, REALM_KEY_CONFIGURATIONS),
	})
}

// endregion: Realms Parser

// region:    Blocks Parser
fn parse_blocks(y_blocks: &Yaml) -> Vec<Block> {
	let blocks = y_blocks
		.as_vec()
		.map(|y_blocks| y_blocks.iter().filter_map(|x| parse_block(x)).collect::<Vec<Block>>());
	blocks.unwrap_or_else(|| Vec::new())
}

fn parse_block(yaml: &Yaml) -> Option<Block> {
	if let Some(name) = yaml.as_str() {
		Some(Block {
			name: name.to_string(),
			..Default::default()
		})
	} else if let Some(name) = yaml["name"].as_str() {
		let mut y_map = yaml.clone();
		y_map = remove_keys(y_map, BLOCK_KEYS);
		// y
		Some(Block {
			name: name.to_string(),
			dir: as_string(&yaml, BLOCK_KEY_DIR),
			dependencies: as_strings(&yaml, BLOCK_KEY_DEP),
			map: as_yaml_map(y_map),
		})
	}
	// if we do not have a name, we ignore
	else {
		None
	}
}
// endregion: Blocks Parser

// region:    Builders Parser
fn parse_builders(y_builders: &Yaml) -> Vec<Builder> {
	let builders = y_builders
		.as_vec()
		.map(|y_builders| y_builders.iter().filter_map(|x| parse_builder(x)).collect::<Vec<Builder>>());
	builders.unwrap_or_else(|| Vec::new())
}

fn parse_builder(yaml: &Yaml) -> Option<Builder> {
	if let Some(name) = yaml["name"].as_str() {
		let exec = match Exec::from_yaml(&yaml["exec"]) {
			Ok(exec) => exec,
			Err(ex) => {
				println!(
					"KDD PARSING WARNING - Builder {} does not have a value exec element. Cause: {}. Skipping",
					name, ex
				);
				return None;
			}
		};

		let when_file = as_string(yaml, "when_file");

		if when_file.is_none() {
			println!(
				"KDD PARSING WARNING - Processor {} does not have an .when_file property. Will never get triggered",
				name
			);
		}

		Some(Builder {
			name: name.to_owned(),
			when_file,
			exec,
		})
	} else {
		None
	}
}
// endregion: Builders Parser
