////////////////////////////////////
// Update the version variables/references in files
////

use regex::Regex;
use std::fs::{read_to_string, write};
use yaml_rust::Yaml;

use super::Kdd;
use crate::{
	app_error::AppError,
	yutils::{as_string, as_strings},
};

//// Version Struct
#[derive(Debug)]
pub struct Version {
	replace: String,
	by: String,
	files: Vec<String>,
}

///// Version Parser
impl Version {
	pub fn from_yaml(yaml: &Yaml) -> Option<Version> {
		let replace = as_string(yaml, "replace");
		let by = as_string(yaml, "by");
		let files = as_strings(yaml, "in");
		if let (Some(replace), Some(by), Some(files)) = (replace, by, files) {
			Some(Version { replace, by, files })
		} else {
			None
		}
	}
}

impl<'a> Kdd<'a> {
	pub fn version(&self) -> Result<(), AppError> {
		println!("========  Versions");
		if self.versions.len() > 0 {
			for version in self.versions.iter() {
				let rx = match Regex::new(&version.replace) {
					Ok(replace) => replace,
					Err(ex) => {
						println!("WARNING - version.replace {} is not valid. Cause: {}. Skip versioning", version.replace, ex);
						continue;
					}
				};
				let by = &version.by;

				for file in version.files.iter() {
					let full_path = self.dir.join(file);

					match read_to_string(&full_path) {
						Ok(content) => {
							let new_content = rx.replace_all(&content, by);
							match write(&full_path, new_content.as_bytes()) {
								Ok(_) => (),
								Err(ex) => {
									println!("WARNING - Cannot write to file {} cause: {}. Skipping version for this file", file, ex);
								}
							}
							println!("Updated file {} with version {}", file, by);
						}
						Err(ex) => {
							println!("WARNING - Cannot read file {} cause: {}. Skipping version for this file", file, ex);
						}
					}
				}
			}
		}
		println!("======== /Versions");
		Ok(())
	}
}
