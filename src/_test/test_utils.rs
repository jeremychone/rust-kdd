use std::{collections::HashMap, path::Path};

use crate::kdd::{error::KddError, Kdd};

const APP_1_DIR: &str = "./test-data/app-1";

pub fn load_kdd() -> Result<Kdd, KddError> {
	let root_dir = Path::new(APP_1_DIR).to_path_buf();
	Kdd::from_dir(root_dir)
}

pub fn get_str<'a>(vars: &'a HashMap<String, String>, key: &str) -> Option<&'a str> {
	vars.get(key).map(|v| &v[..])
}
