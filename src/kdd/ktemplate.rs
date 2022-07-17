////////////////////////////////
// kdd::kube - impls for all kubenetes relative actions
// --

use std::{
	collections::HashMap,
	fs::{create_dir_all, read_to_string, File},
	io::Write,
	path::PathBuf,
};

use handlebars::{Handlebars, RenderError};
use pathdiff::diff_paths;

use super::{error::KddError, realm::Realm, Kdd};

impl Kdd {
	pub fn k_templates(&self, realm: &Realm, names: Option<&[&str]>, print_full: bool) -> Result<Vec<PathBuf>, KddError> {
		let k8s_files = realm.k8s_files(names);
		let mut k8s_out_files: Vec<PathBuf> = Vec::new();

		let out_dir = realm.k8s_out_dir();
		if !out_dir.is_dir() {
			create_dir_all(&out_dir)?;
		}

		// -- take the kdd vars and merge the realm var on top of it
		let mut merged_vars = self.vars.clone();
		for (name, val) in realm.vars.iter() {
			merged_vars.insert(name.to_string(), val.to_string());
		}
		let merged_vars = merged_vars;

		// -- render the files
		if print_full {
			println!("---  Rendering yaml files");
		}
		let hbs: Handlebars = Handlebars::new();

		for src_file in k8s_files {
			if let Some(file_name) = src_file.file_name().map(|v| v.to_str()).flatten() {
				// -- render the content
				let src_file_rel_path = diff_paths(&src_file, &self.dir).unwrap();
				let src_content = read_to_string(&src_file)?;

				let out_content = match self.k_render_file(&hbs, &src_content, &merged_vars) {
					Ok(v) => v,
					Err(ex) => {
						return Err(KddError::KtemplateFailRender(
							src_file_rel_path.to_string_lossy().to_string(),
							ex.to_string(),
						))
					}
				};

				let out_path = out_dir.join(file_name);
				let mut out_file = File::create(&out_path)?;
				out_file.write_all(out_content.as_bytes())?;

				let out_file_rel_path = diff_paths(&out_path, &self.dir).unwrap();
				if print_full {
					println!(
						"{:<28}>>>  {}",
						src_file_rel_path.to_string_lossy(),
						out_file_rel_path.to_string_lossy()
					);
				}
				k8s_out_files.push(out_path);
			}
		}
		if print_full {
			println!("--- /Rendering yaml files - DONE\n");
		} else {
			println!("Rendered {} yaml files - DONE\n", k8s_out_files.len());
		}

		Ok(k8s_out_files)
	}

	fn k_render_file(&self, hbs: &Handlebars<'_>, src_content: &str, vars: &HashMap<String, String>) -> Result<String, RenderError> {
		hbs.render_template(src_content, vars)
	}
}
