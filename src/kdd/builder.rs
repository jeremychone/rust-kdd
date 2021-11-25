////////////////////////////////////
// kdev::builder - The the kdd Builder component and its Exec component
////

use super::error::KddError;
use crate::{
	utils::path_to_string,
	utils::yamls::{as_string, as_strings},
};
use pathdiff::diff_paths;
use std::path::Path;
use tokio::process::{Child, Command};
use yaml_rust::Yaml;

//// Builder Struct
#[derive(Debug)]
pub struct Builder {
	pub name: String,
	pub when_file: Option<String>,
	pub replace: Option<String>,
	pub exec: Exec,
}

//// Builder Maker
impl Builder {
	pub fn from_yaml(yaml: &Yaml) -> Option<Builder> {
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

			let replace = as_string(yaml, "replace");

			if when_file.is_none() {
				println!(
					"KDD PARSING WARNING - Processor {} does not have an .when_file property. Will never get triggered",
					name
				);
			}

			Some(Builder {
				name: name.to_owned(),
				replace,
				when_file,
				exec,
			})
		} else {
			None
		}
	}
}

// region:    Exec Component
#[derive(Debug, Clone)]
pub struct Exec {
	cmd: Cmd, // from base dir if not prefixed, if prefixed with ./ then block_dir is prefixed
	args: Vec<String>,
	watch_args: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
enum Cmd {
	Global(String),   // global cmd, e.g., npm
	Base(String),     // from kdd base dir, e.g., node_modules/.bin/tsc
	Relative(String), // local to entity (e.g., block) e.g. ./module/.bin/
}

//// Exec Builder(s)
impl Exec {
	pub fn from_yaml(y_exec: &Yaml) -> Result<Self, KddError> {
		let cmd_name = as_string(&y_exec, "cmd").ok_or_else(|| KddError::NoExecCmd)?;

		let cmd = if cmd_name.starts_with("./") {
			Cmd::Relative(cmd_name) // relative to entity (.e.g., block.dir)
		} else if cmd_name.contains("/") {
			// e.g., node_modules/.bin/tsc, relative to base dir, so willadd the ../..
			Cmd::Base(cmd_name) // relative to kdd base dir
		} else {
			Cmd::Global(cmd_name)
		};

		let args = as_strings(&y_exec, "args").unwrap_or_else(|| Vec::new());
		let watch_args = as_strings(&y_exec, "watch_args");
		Ok(Exec { cmd, args, watch_args })
	}
}

//// Exec Public Methods
impl Exec {
	pub async fn execute_and_wait(&self, kdd_dir: &Path, block_dir: &Path, watch: bool) -> Result<(), KddError> {
		let mut proc = self.execute(kdd_dir, block_dir, watch)?;
		// proc.wait()?

		match proc.wait().await {
			Ok(_) => Ok(()),
			Err(ex) => Err(KddError::CannotExecute(ex.to_string())),
		}
	}

	pub fn execute(&self, kdd_dir: &Path, block_dir: &Path, watch: bool) -> Result<Child, KddError> {
		let cwd = block_dir;

		let cmd = match &self.cmd {
			// e.g., npm
			Cmd::Global(val) => val.to_string(),
			// e.g., ./node_module/.bin/ (from block dir)
			Cmd::Relative(val) => val.to_string(),
			// e.g., node_modules/.bin/tsc (those need to be prefix to point back to base dir)
			Cmd::Base(val) => {
				// TODO: Needs to handle those unwrap eventually
				let diff = diff_paths(kdd_dir, cwd).unwrap();
				let path = diff.join(val);
				path_to_string(&path).unwrap()
			}
		};

		// args
		let args = match (watch, &self.watch_args) {
			(true, Some(watch_args)) => &watch_args[..],
			_ => &self.args[..],
		};

		// build proc
		let mut proc = Command::new(&cmd);
		proc.current_dir(&cwd);
		proc.args(args);

		// execute
		println!("> executing: {} {} (at cwd: {})  ", cmd, args.join(" "), cwd.to_string_lossy(),);
		match proc.spawn() {
			Ok(proc) => Ok(proc),
			Err(ex) => {
				println!("  ERROR - Fail to execute. Cause: {}", ex);
				Err(KddError::CannotExecute(ex.to_string()))
			}
		}
	}
}
// endregion: Exec Component
