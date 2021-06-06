use std::{path::Path, process::Command};

use yaml_rust::Yaml;

use crate::{
	utils::path_to_string,
	yutils::{as_string, as_strings},
};

use super::error::KddError;
use pathdiff::diff_paths;

#[derive(Debug)]
enum Cmd {
	Global(String),   // global cmd, e.g., npm
	Base(String),     // from kdd base dir, e.g., node_modules/.bin/tsc
	Relative(String), // local to entity (e.g., block) e.g. ./module/.bin/
}

#[derive(Debug)]
pub struct Exec {
	cmd: Cmd, // from base dir if not prefixed, if prefixed with ./ then block_dir is prefixed
	args: Vec<String>,
}

// region:    Exec From
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

		Ok(Exec { cmd, args })
	}
}
// endregion: Exec From

// region:    Exec Impls
impl Exec {
	pub fn execute(&self, kdd_dir: &Path, block_dir: &Path) {
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

		// proc
		let args = &self.args[..];
		let mut proc = Command::new(&cmd);
		proc.current_dir(&cwd);
		proc.args(args);

		println!("> executing: {} {} (at cwd: {})  ", cmd, args.join(" "), cwd.to_string_lossy(),);

		let mut proc = match proc.spawn() {
			Ok(proc) => proc,
			Err(ex) => {
				println!("  ERROR - Fail to execute. Cause: {}", ex);
				return;
			}
		};

		match proc.wait() {
			Ok(_) => return,
			Err(ex) => {
				println!("  ERROR - Faild during execution. Cause: {}", ex);
			}
		}
	}
}
// endregion: Exec Impls
