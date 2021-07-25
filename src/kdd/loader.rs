////////////////////////////////////
// kdd::loader - Responsible to load and instantiate a kdd
////

use std::{collections::HashMap, env, fs::read_to_string, path::PathBuf};

use handlebars::Handlebars;
use indexmap::IndexMap;
use regex::Regex;
use yaml_rust::{Yaml, YamlLoader};

use crate::{
	utils::{has_prop, path_to_string},
	yutils::{as_string, merge_yaml},
};

use super::{error::KddError, Block, Builder, Kdd, Realm};
use serde_json::Value;

const KDD_KEY_SYSTEM: &str = "system";
const KDD_KEY_BLOCK_DIR: &str = "block_base_dir";
const KDD_KEY_IMAGE_TAG: &str = "image_tag";

// Kdev Builder
impl<'a> Kdd<'a> {
	pub fn from_dir(dir: PathBuf) -> Result<Kdd<'a>, KddError> {
		//// build the template engine
		let hbs: Handlebars = Handlebars::new();

		//// base vars
		let mut vars: HashMap<String, String> = HashMap::new();
		vars.insert("dir".to_owned(), path_to_string(&dir)?);
		vars.insert("dir_abs".to_owned(), path_to_string(&dir.canonicalize()?)?);

		//// load the root yaml
		let kdd_path = dir.join("kdd.yaml");
		if !kdd_path.is_file() {
			return Err(KddError::NoKdevFileFound(dir.to_string_lossy().to_string()));
		}
		let kdd_content = read_to_string(kdd_path)?;
		let rx = Regex::new(r"(?m)^---.*\W").expect("works once, works all the time");
		let splits: Vec<_> = rx.split(&kdd_content).collect();
		let (kdd_yaml_txt, extra_vars) = match splits.len() {
			// if only one template, then, just the core kdd template and empty vars
			1 => (splits[0], None),
			// if two yaml document, first ones is the vars and second is the kdd template
			2 => {
				let vars = load_vars(&dir, YamlLoader::load_from_str(splits[0])?);
				(splits[1], Some(vars))
			}
			// otherwise, fail for now
			_ => {
				return Err(KddError::KdevYamlInvalid);
			}
		};

		// add eventual extra vars (from first yaml doc)
		if let Some(extra_vars) = extra_vars {
			// consume the extra_vars
			for (name, val) in extra_vars.into_iter() {
				vars.insert(name, val);
			}
		}

		// handlebars process the kdd yaml text
		let rendered_yaml = match hbs.render_template(kdd_yaml_txt, &vars) {
			Ok(r) => r,
			Err(e) => return Err(KddError::KdevFailToParseInvalid(e.to_string())),
		};
		let kdd_yaml = YamlLoader::load_from_str(&rendered_yaml)?;

		let kdd_yaml = &kdd_yaml[0];

		//// load the base properties
		let system = as_string(kdd_yaml, KDD_KEY_SYSTEM).ok_or(KddError::NoSystem)?;
		vars.insert("system".to_owned(), system.to_string());

		//// read the blocks
		let blocks = parse_blocks(&kdd_yaml["blocks"]);

		//// read the realms
		let realms = parse_realms(&dir, &kdd_yaml["realms"]);

		//// read the builder
		let builders = parse_builders(&kdd_yaml["builders"]);

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
enum FileVarsSource {
	Json(PathBuf),
	NotSupported(PathBuf),
}

impl FileVarsSource {
	fn from_path(path: PathBuf) -> FileVarsSource {
		if let Some(Some(ext)) = path.extension().map(|v| v.to_str().map(|v| v.to_lowercase())) {
			match ext.as_str() {
				"json" => FileVarsSource::Json(path),
				_ => FileVarsSource::NotSupported(path),
			}
		} else {
			FileVarsSource::NotSupported(path)
		}
	}
}

fn load_vars(dir: &PathBuf, yamls: Vec<Yaml>) -> HashMap<String, String> {
	let mut vars: HashMap<String, String> = HashMap::new();

	for yaml in yamls.iter() {
		if let Some(vars_yaml) = yaml["vars"].as_vec() {
			for yaml_item in vars_yaml.iter() {
				match (has_prop(&yaml_item, "from_file"), has_prop(&yaml_item, "from_env")) {
					(Some(from_file_yaml), None) => load_vars_from_file(dir, from_file_yaml, &mut vars),
					(None, Some(from_env_yaml)) => load_vars_from_env(from_env_yaml, &mut vars),
					(None, None) => println!("KDD WARNING - no valid vars yaml item. Skip."),
					(Some(_), Some(_)) => println!("KDD WARNING - vars items cannot have from_file and from_env. Skip"),
				}
			}
		}
	}
	vars
}

fn load_vars_from_env(yaml_item: &Yaml, vars: &mut HashMap<String, String>) {
	if let Some(items) = yaml_item["from_env"].as_vec() {
		for name in items.iter() {
			if let Some(name) = name.as_str() {
				if let Ok(val) = env::var(name) {
					vars.insert(name.to_owned(), val);
				}
			}
		}
	}
}

fn load_vars_from_file(dir: &PathBuf, yaml_item: &Yaml, vars: &mut HashMap<String, String>) {
	if let (Some(extract), Some(file)) = (yaml_item["extract"].as_vec(), yaml_item["from_file"].as_str()) {
		match FileVarsSource::from_path(dir.join(file)) {
			FileVarsSource::Json(path) => match read_to_string(&path) {
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
			FileVarsSource::NotSupported(path) => {
				println!("KDD WARNING - file {} not supported as a variable source. - SKIP", path.to_string_lossy());
			}
		}
	}
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

					match Realm::from_yaml(kdd_dir, name, yaml_data) {
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

// endregion: Realms Parser

// region:    Blocks Parser
fn parse_blocks(y_blocks: &Yaml) -> Vec<Block> {
	let blocks = y_blocks
		.as_vec()
		.map(|y_blocks| y_blocks.iter().filter_map(|x| Block::from_yaml(x)).collect::<Vec<Block>>());
	blocks.unwrap_or_else(|| Vec::new())
}

// endregion: Blocks Parser

// region:    Builders Parser
fn parse_builders(y_builders: &Yaml) -> Vec<Builder> {
	let builders = y_builders
		.as_vec()
		.map(|y_builders| y_builders.iter().filter_map(|x| Builder::from_yaml(x)).collect::<Vec<Builder>>());
	builders.unwrap_or_else(|| Vec::new())
}

// endregion: Builders Parser
