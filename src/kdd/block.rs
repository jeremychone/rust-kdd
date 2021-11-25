////////////////////////////////////
// kdd::realm - All realm related actions
////

use yaml_rust::Yaml;

use crate::utils::yamls::{as_string, as_strings, as_yaml_map, remove_keys};

const BLOCK_KEY_NAME: &str = "name";
const BLOCK_KEY_DIR: &str = "dir";
const BLOCK_KEY_DEP: &str = "dependencies";
const BLOCK_KEYS: &[&str] = &[BLOCK_KEY_NAME, BLOCK_KEY_DIR, BLOCK_KEY_DEP];

//// Block Struct
#[derive(Debug, Default)]
pub struct Block {
	pub name: String,
	pub dir: Option<String>,
	pub dependencies: Option<Vec<String>>,
	pub map: Option<Yaml>,
}

//// Block Builder(s)
impl Block {
	pub fn from_yaml(yaml: &Yaml) -> Option<Block> {
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
}
