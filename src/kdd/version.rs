////////////////////////////////////
// Update the version variables/references in files
////

use regex::Regex;
use std::fs::{read_to_string, write};
use yaml_rust::Yaml;

use crate::app_error::AppError;
use crate::utils::yamls::{as_string, as_strings};

use super::Kdd;

//// Version Struct
#[derive(Debug)]
pub struct Version {
	val: String,
	replace: String,
	by: String,
	files: Vec<String>,
}

///// Version Parser
impl Version {
	pub fn from_yaml(yaml: &Yaml) -> Option<Version> {
		let val = as_string(yaml, "val");
		let replace = as_string(yaml, "replace");
		let by = as_string(yaml, "by");
		let files = as_strings(yaml, "in");
		if let (Some(val), Some(replace), Some(by), Some(files)) = (val, replace, by, files) {
			Some(Version { val, replace, by, files })
		} else {
			None
		}
	}
}

impl<'a> Kdd<'a> {
	pub fn version(&self, out: &mut impl std::io::Write) -> Result<(), AppError> {
		writeln!(out, "========  Versions")?;
		if self.versions.len() > 0 {
			for version in self.versions.iter() {
				let val_rgx = match Regex::new(&version.val) {
					Ok(val) => val,
					Err(ex) => {
						writeln!(
							out,
							"WARNING - version.val {} is not valid. Cause: {}. Skip versioning",
							version.val, ex
						)?;
						continue;
					}
				};
				let replace_rgx = match Regex::new(&version.replace) {
					Ok(replace) => replace,
					Err(ex) => {
						writeln!(
							out,
							"WARNING - version.replace {} is not valid. Cause: {}. Skip versioning",
							version.replace, ex
						)?;
						continue;
					}
				};
				let by = &version.by;

				for file in version.files.iter() {
					let full_path = self.dir.join(file);

					match read_to_string(&full_path) {
						Ok(content) => {
							// extract original value
							let org_val = val_rgx
								.captures(&content)
								.map(|caps| caps.get(caps.len() - 1).map(|m| m.as_str()))
								.flatten();
							// replace the content
							let content = replace_rgx.replace_all(&content, by);

							// extract the new value
							let new_val = val_rgx
								.captures(&content)
								.map(|caps| caps.get(caps.len() - 1).map(|m| m.as_str()))
								.flatten();

							// write to the file
							match write(&full_path, content.as_bytes()) {
								Ok(_) => (),
								Err(ex) => {
									writeln!(
										out,
										"WARNING - Cannot write to file {} cause: {}. Skipping version for this file",
										file, ex
									)?;
								}
							}
							// write info
							writeln!(
								out,
								"Updated version '{}' to '{}' in file {}",
								org_val.unwrap_or("NO_VAL"),
								new_val.unwrap_or("NO_VAL"),
								file
							)?;
						}
						Err(ex) => {
							writeln!(
								out,
								"WARNING - Cannot read file {} cause: {}. Skipping version for this file",
								file, ex
							)?;
						}
					}
				}
			}
		}
		writeln!(out, "======== /Versions")?;
		Ok(())
	}
}

// region:    Tests
#[cfg(test)]
#[path = "../_test/kdd_version.rs"]
mod tests;
// endregion: Tests
