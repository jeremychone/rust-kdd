////////////////////////////////////
// kdd::provider - cloud specific adapters
////

use super::{error::KddError, Block, Realm};
use crate::utils::{exec_cmd_args, exec_to_stdout};

use core::fmt::Debug;
use serde_json::Value;
use std::{
	collections::HashSet,
	io::Write,
	process::{Command, Stdio},
};
use strum_macros::Display;

/// Default empty implementation for cloud/cluster providers.
pub trait Provider {
	/// Called to check if the Realm is valid for cloud Provider
	fn check_realm(&self) -> Result<(), KddError> {
		Ok(())
	}

	fn before_set_realm(&self) -> Result<(), KddError> {
		Ok(())
	}

	fn before_dpushes(&self, _system: &str, _realmm: &Realm, _blocks: &[&Block]) -> Result<(), KddError> {
		Ok(())
	}

	fn docker_auth(&self, _realm: &Realm) -> Result<(), KddError> {
		Ok(())
	}
}

#[derive(Debug)]
pub struct AwsProvider;

#[derive(Debug)]
pub struct GcpProvider;

#[derive(Debug)]
pub struct CommonProvider;

#[derive(Debug, Display)]
pub enum RealmProvider {
	Aws(AwsProvider),
	Gcp(GcpProvider),
	Common(CommonProvider),
}

impl Debug for dyn Provider {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "Provider{{}}")
	}
}

// region:    Aws Provider
impl Provider for AwsProvider {
	fn before_dpushes(&self, system: &str, realm: &Realm, blocks: &[&Block]) -> Result<(), KddError> {
		let existing_repo_names = self.get_aws_repository_names(realm)?;
		let block_repo_names: Vec<String> = blocks.iter().map(|b| format!("{}-{}", system, b.name)).collect();
		let existing_repo_names: HashSet<&String> = existing_repo_names.iter().collect();
		let block_repo_names: HashSet<&String> = block_repo_names.iter().collect();

		for name in block_repo_names.into_iter() {
			if !existing_repo_names.contains(name) {
				//aws ecr create-repository --profile jc-root --repository-name cstar-agent

				exec_cmd_args(
					None,
					"aws",
					&["ecr", "create-repository", "--profile", &realm.profile(), "--repository-name", name],
				)?;
			}
		}
		Ok(())
	}

	fn docker_auth(&self, realm: &Realm) -> Result<(), KddError> {
		if let Some(registry) = &realm.registry {
			// get the password
			let pwd = exec_to_stdout(None, "aws", &["ecr", "get-login-password", "--profile", &realm.profile()], false)?;

			// execute the login
			let cmd = "docker";
			let args = &["login", "--username", "AWS", "--password-stdin", registry];
			println!("> executing: {} {} (with previous command result as stdin)", cmd, args.join(" "));
			let mut proc = Command::new(&cmd);
			proc.args(args);
			proc.stdin(Stdio::piped());
			let mut child = proc.spawn()?;
			child.stdin.as_ref().unwrap().write_all(pwd.as_bytes())?;
			child.wait()?;
		}

		Ok(())
	}
}

impl AwsProvider {
	pub fn get_aws_repository_names(&self, realm: &Realm) -> Result<Vec<String>, KddError> {
		let mut names: Vec<String> = Vec::new();
		let profile = realm.profile();

		// aws ecr describe-repositories --profile jc-root
		let json = exec_to_stdout(None, "aws", &["ecr", "describe-repositories", "--profile", &profile], false)?;
		let json = serde_json::from_str::<Value>(&json).map_err(|ex| KddError::AwsEcrDescribeRepositoriesFailed(ex.to_string()))?;
		if let Some(reps) = json["repositories"].as_array() {
			for rep in reps {
				if let Some(name) = rep["repositoryName"].as_str() {
					names.push(name.to_string());
				}
			}
		}

		Ok(names)
	}
}
// endregion: Aws Provider

// region:    Gcp Provider
impl Provider for GcpProvider {}
// endregion: Gcp Provider

// region:    Common Provider
impl Provider for CommonProvider {}
// endregion: Common Provider
