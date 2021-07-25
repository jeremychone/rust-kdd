use std::{
	io::Error as IOError,
	path::PathBuf,
	process::{Command, ExitStatus, Stdio},
};
use thiserror::Error;
use yaml_rust::Yaml;

#[derive(Error, Debug)]
pub enum UtilsError {
	#[error("Fail to execute {0} cause: {1}")]
	ExecError(String, String),

	#[error("Path '{0}' (lossy representation) seems to not be utf8")]
	PathNotUtf8(String),
}

impl UtilsError {
	fn from_exec_stderr(cmd: &str, args: &[&str], cause: &dyn std::error::Error) -> Self {
		let command = format!("{} {}", cmd, args.join(" "));
		UtilsError::ExecError(command, cause.to_string())
	}
	fn from_exec_status(cmd: &str, args: &[&str], status: ExitStatus) -> Self {
		let command = format!("{} {}", cmd, args.join(" "));
		UtilsError::ExecError(command, status.to_string())
	}
}

/// extract the utf8 string of a PathBuf, and throw error if not possible.
#[inline]
pub fn path_to_string(path: &PathBuf) -> Result<String, UtilsError> {
	match path.to_str().map(|p| p.to_string()) {
		Some(path) => Ok(path),
		None => Err(UtilsError::PathNotUtf8(path.to_string_lossy().to_string())),
	}
}

pub fn exec_proc(proc: &mut Command) -> Result<ExitStatus, IOError> {
	Ok(proc.spawn()?.wait()?)
}

pub fn exec_cmd_args(cwd: Option<&PathBuf>, cmd: &str, args: &[&str]) -> Result<(), UtilsError> {
	let mut proc = Command::new(cmd);
	if let Some(cwd) = cwd {
		proc.current_dir(cwd);
	}
	proc.args(args);

	println!("> executing: {} {}", cmd, args.join(" "));

	match exec_proc(&mut proc) {
		Ok(status) => {
			if !status.success() {
				Err(UtilsError::from_exec_status(cmd, args, status))
			} else {
				Ok(())
			}
		}
		Err(ex) => Err(UtilsError::from_exec_stderr(cmd, args, &ex)),
	}
}

pub fn exec_to_stdout(cwd: Option<&PathBuf>, cmd: &str, args: &[&str], print_exec: bool) -> Result<String, UtilsError> {
	if print_exec {
		println!("> executing: {} {}", cmd, args.join(" "));
	}
	let mut proc = Command::new(&cmd);
	if let Some(cwd) = cwd {
		proc.current_dir(cwd);
	}
	proc.args(args);
	match proc.stdout(Stdio::piped()).output() {
		Err(ex) => Err(UtilsError::from_exec_stderr(cmd, args, &ex)),
		Ok(output) => {
			let txt = if output.status.success() {
				String::from_utf8(output.stdout)
			} else {
				String::from_utf8(output.stderr)
			};

			match txt {
				Err(ex) => Err(UtilsError::from_exec_stderr(cmd, args, &ex)),
				Ok(txt) => Ok(txt),
			}
		}
	}
}

// region:    Yaml Utils
/// Check if the yaml has a given prop, and if does, just self as
/// Some(Yaml) otherwise returns None
pub fn has_prop<'a>(yaml: &'a Yaml, prop_name: &str) -> Option<&'a Yaml> {
	if yaml[prop_name].is_badvalue() {
		None
	} else {
		Some(yaml)
	}
}
// endregion: Yaml Utils
