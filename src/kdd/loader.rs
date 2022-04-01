//
// kdd::loader - Responsible to load and instantiate a kdd
// --

use super::{error::KddError, version::Version, Block, Builder, Kdd, Realm};
use crate::utils::yamls::{as_string, as_strings, merge_yaml, print_yaml};
use crate::utils::{has_prop, path_to_string};
use handlebars::Handlebars;
use indexmap::IndexMap;
use regex::Regex;
use serde_json::Value;
use std::{collections::HashMap, env, fs::read_to_string, path::PathBuf};
use yaml_rust::{Yaml, YamlLoader};

const KDD_KEY_SYSTEM: &str = "system";
const KDD_KEY_BLOCK_DIR: &str = "block_base_dir";
const KDD_KEY_IMAGE_TAG: &str = "image_tag";

// Kdev Builder
impl<'a> Kdd<'a> {
	pub fn from_dir(dir: PathBuf) -> Result<Kdd<'a>, KddError> {
		// -- build the template engine
		let hbs: Handlebars = Handlebars::new();

		// -- root vars
		let mut root_vars: HashMap<String, String> = HashMap::new();
		root_vars.insert("dir".to_owned(), path_to_string(&dir)?);
		root_vars.insert("dir_abs".to_owned(), path_to_string(&dir.canonicalize()?)?);

		// -- load main KddPart
		let kdd_path = dir.join("kdd.yaml");
		if !kdd_path.is_file() {
			return Err(KddError::NoKddFileFound(dir.to_string_lossy().to_string()));
		}
		let kdd_content = read_to_string(kdd_path)?;
		let KddRawPart {
			kdd_yaml_txt,
			vars: extra_vars,
			overlays,
		} = parse_kdd_raw_part(&dir, &kdd_content)?;

		// add to root vars
		merge_vars(&mut root_vars, extra_vars);

		let kdd_part = parse_kdd_part(&dir, &kdd_yaml_txt, &mut root_vars, &hbs, &None)?;

		let KddPart {
			kdd_yaml,
			blocks,
			builders,
			versions,
			system,
			realm_base,
			..
		} = kdd_part;
		let mut realms = kdd_part.realms;

		// extract system variable and set as var
		let system = system.ok_or(KddError::NoSystem)?;
		root_vars.insert("system".to_owned(), system.to_string());

		// -- merge the overlay
		for overlay_yaml_txt in overlays.into_iter() {
			let KddRawPart {
				kdd_yaml_txt: overlay_kdd_yaml_txt,
				vars: extra_vars,
				..
			} = parse_kdd_raw_part(&dir, &overlay_yaml_txt)?;

			merge_vars(&mut root_vars, extra_vars);

			// parse the overlay kdd yaml
			let overlay_kdd_part = parse_kdd_part(&dir, &overlay_kdd_yaml_txt, &mut root_vars, &hbs, &realm_base)?;

			// overlay the new realms
			let KddPart {
				realms: overlay_realms, ..
			} = overlay_kdd_part;
			for (name, realm) in overlay_realms.into_iter() {
				realms.insert(name, realm);
			}
		}

		// -- build final kdd
		let kdd = Kdd {
			hbs,
			vars: root_vars,
			dir,
			system,
			// both has to come from first kdd_yaml
			block_base_dir: as_string(&kdd_yaml, KDD_KEY_BLOCK_DIR),
			image_tag: as_string(&kdd_yaml, KDD_KEY_IMAGE_TAG),
			blocks,
			realms,
			builders,
			versions,
		};

		Ok(kdd)
	}
}

// region:    KddPart Parsing
struct KddRawPart {
	/// Main kdd yaml raw text
	kdd_yaml_txt: String,
	/// Vars from the eventual yaml_pre
	vars: HashMap<String, String>,
	/// raw yaml document(s) of the eventual overlays content in yaml_pre.overlays
	overlays: Vec<String>,
}

fn parse_kdd_raw_part(dir: &PathBuf, kdd_content: &str) -> Result<KddRawPart, KddError> {
	let mut vars: HashMap<String, String> = HashMap::new();

	let rx = Regex::new(r"(?m)^---.*\W").expect("works once, works all the time");
	let splits: Vec<_> = rx.split(&kdd_content).collect();
	let (kdd_yaml_txt, pre_yaml_txt) = match splits.len() {
		// if only one template, then, just the core kdd template and empty vars
		1 => (splits[0].to_owned(), None),
		// if two yaml document, first ones is the vars and second is the kdd template
		2 => (splits[1].to_owned(), Some(splits[0])),
		// otherwise, fail for now
		_ => {
			return Err(KddError::KddYamlInvalid);
		}
	};

	let (extra_vars, overlays) = match pre_yaml_txt {
		None => (None, Vec::new()),
		Some(pre_yaml) => {
			let pre_yaml = YamlLoader::load_from_str(pre_yaml)?;
			let extra_vars = load_vars(&dir, &pre_yaml);
			let overlays = load_overlays(&dir, &pre_yaml);
			(Some(extra_vars), overlays)
		}
	};

	// add eventual extra vars (from first yaml doc)
	if let Some(extra_vars) = extra_vars {
		// consume the extra_vars
		for (name, val) in extra_vars.into_iter() {
			vars.insert(name, val);
		}
	}

	Ok(KddRawPart {
		kdd_yaml_txt,
		vars,
		overlays,
	})
}

struct KddPart {
	system: Option<String>,
	blocks: Vec<Block>,
	realms: IndexMap<String, Realm>,
	realm_base: Option<Yaml>,
	builders: Vec<Builder>,
	versions: Vec<Version>,
	kdd_yaml: Yaml,
}

fn parse_kdd_part(
	dir: &PathBuf,
	kdd_yaml_txt: &str,
	root_vars: &mut HashMap<String, String>,
	hbs: &Handlebars,
	realm_root_base: &Option<Yaml>,
) -> Result<KddPart, KddError> {
	// handlebars process the kdd yaml text
	let rendered_yaml = match hbs.render_template(&kdd_yaml_txt, &root_vars) {
		Ok(r) => r,
		Err(e) => return Err(KddError::KdevFailToParseInvalid(e.to_string())),
	};
	let mut kdd_yaml = YamlLoader::load_from_str(&rendered_yaml)?;

	let kdd_yaml = kdd_yaml.remove(0);

	// -- load the base properties
	let system = as_string(&kdd_yaml, KDD_KEY_SYSTEM);

	// -- read the blocks
	let blocks = parse_blocks(&kdd_yaml["blocks"]);

	// -- read the realms
	let (realm_base, realms) = parse_realms(dir, &kdd_yaml["realms"], realm_root_base);

	// -- read the builders
	let builders = parse_builders(&kdd_yaml["builders"]);

	// -- read the versions
	let versions = parser_versions(&kdd_yaml["versions"]);

	// add all of the root variables as vars
	if let Some(map) = kdd_yaml.as_hash() {
		for (name, val) in map.iter() {
			if let (Some(name), Some(val)) = (name.as_str(), val.as_str()) {
				root_vars.insert(name.to_owned(), val.to_owned());
			}
		}
	}

	Ok(KddPart {
		system,
		blocks,
		realms,
		realm_base,
		builders,
		versions,
		kdd_yaml,
	})
}

fn merge_vars(root_vars: &mut HashMap<String, String>, vars: HashMap<String, String>) {
	for (name, val) in vars.into_iter() {
		root_vars.insert(name, val);
	}
}
// endregion: KddPart Parsing

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

fn load_vars(dir: &PathBuf, yamls: &Vec<Yaml>) -> HashMap<String, String> {
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
				println!(
					"KDD WARNING - file {} not supported as a variable source. - SKIP",
					path.to_string_lossy()
				);
			}
		}
	}
}
// endregion: Load Vars

// region:    Load Overlays
fn load_overlays(dir: &PathBuf, pre_yamls: &Vec<Yaml>) -> Vec<String> {
	let mut overlays: Vec<String> = Vec::new();

	// for now, supports only first doc
	for pre_yaml in pre_yamls.iter() {
		if let Some(files) = as_strings(pre_yaml, "overlays") {
			for file in files {
				if let Ok(content) = read_to_string(dir.join(&file)) {
					println!("KDD INFO - overlay file {} loaded", file);
					overlays.push(content);
				}
			}
			// empty line
			if overlays.len() > 0 {
				println!();
			}
		}
	}

	overlays
}
// endregion: Load Overlays

// region:    Realms Parser
fn parse_realms(kdd_dir: &PathBuf, y_realms: &Yaml, realms_base_vars: &Option<Yaml>) -> (Option<Yaml>, IndexMap<String, Realm>) {
	match y_realms.as_hash() {
		None => (None, IndexMap::new()),
		Some(y_realms) => {
			// load the eventual _base_ properties
			let base = y_realms.get(&Yaml::String("_base_".to_string())).map(|y| y.to_owned());

			let mut realms: IndexMap<String, Realm> = IndexMap::new();
			for y_realm in y_realms.into_iter() {
				let (name, data) = y_realm;
				if let Some(name) = name.as_str() {
					// if name is _base_ then not a realm, continue
					if name == "_base_" {
						continue;
					}

					let mut data = data.clone();

					// -- merge the realms_base_vars if present
					//    Note: This usually means it was the realms._base_ was already loaded in the main kdd.yaml parse
					//    and this is a sub kdd file
					if let Some(realms_base_vars) = realms_base_vars {
						merge_yaml(&mut data, realms_base_vars, true);
					}

					// -- merge the data from _base_ and this realm
					//    Note: This usually means the realm is in the main kdd and this is merged
					if let Some(base) = base.as_ref() {
						merge_yaml(&mut data, &base, true);
					}

					match Realm::from_yaml(kdd_dir, name, &data) {
						Ok(realm) => {
							realms.insert(name.to_string(), realm);
						}
						Err(ex) => println!("KDD ERROR - Fail to parse realm {}. Cause: {}", name, ex),
					}
				}
			}
			(base, realms)
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

// region:    Version Parser
fn parser_versions(y_versions: &Yaml) -> Vec<Version> {
	let versions = y_versions
		.as_vec()
		.map(|y_versions| y_versions.iter().filter_map(|x| Version::from_yaml(x)).collect::<Vec<Version>>());

	versions.unwrap_or_else(|| Vec::new())
}
// endregion: Version Parser

// region:    Test
#[cfg(test)]
#[path = "../_test/kdd_loader.rs"]
mod tests;
// endregion: Test
