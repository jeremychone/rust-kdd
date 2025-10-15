////////////////////////////////////
// kdd::kube - impls for all kubenetes relative actions
////

use std::io::stdin;

use super::{error::KddError, Kdd, Realm};
use crate::utils::{exec_cmd_args, exec_to_stdout, path_to_string};

impl Kdd {
	pub fn k_apply(&self, realm: &Realm, names: Option<&[&str]>) -> Result<(), KddError> {
		let k8s_out_files = self.k_templates(realm, names, false)?;

		for file in k8s_out_files {
			exec_cmd_args(Some(&self.dir), "kubectl", &["apply", "-f", &path_to_string(&file)?])?;
			println!();
		}

		Ok(())
	}

	pub fn k_create(&self, realm: &Realm, names: Option<&[&str]>) -> Result<(), KddError> {
		let k8s_out_files = self.k_templates(realm, names, false)?;

		for file in k8s_out_files {
			exec_cmd_args(Some(&self.dir), "kubectl", &["create", "-f", &path_to_string(&file)?])?;
			println!();
		}

		Ok(())
	}

	#[allow(unused_must_use)] // for ignoring the error on exec_cmd_args
	pub fn k_delete(&self, realm: &Realm, names: Option<&[&str]>) -> Result<(), KddError> {
		let k8s_out_files = self.k_templates(realm, names, false)?;

		if realm.confirm_delete {
			println!("Are you sure you want to delete the services in realm {}? (YES to continue, anything else to cancel)", realm.name);
			let mut guess = String::new();
			stdin().read_line(&mut guess).expect("Failed to read line");
			if guess.trim() != "YES" {
				println!("Canceling kubectl delete");
				return Ok(());
			}
		}

		for file in k8s_out_files {
			// Note: no need to handle error, it will print, and we want to skip to next
			exec_cmd_args(Some(&self.dir), "kubectl", &["delete", "-f", &path_to_string(&file)?]);
			println!();
		}

		Ok(())
	}

	pub fn k_create_ctx(&self, ctx: &str) -> Result<(), KddError> {
		match exec_to_stdout(Some(&self.dir), "kubectl", &["config", "set-context", ctx], false) {
			Ok(_) => Ok(()),
			Err(e) => Err(KddError::KubectlFail(e.to_string())),
		}
	}

	pub fn k_list_ctx(&self) -> Result<Vec<String>, KddError> {
		match exec_to_stdout(Some(&self.dir), "kubectl", &["config", "get-contexts", "-o=name"], false) {
			Ok(ctxs) => Ok(ctxs.lines().map(|s| s.trim().to_string()).collect()),
			Err(e) => Err(KddError::KubectlFail(e.to_string())),
		}
	}

	pub fn k_delete_ctx(&self, ctx: &str) -> Result<(), KddError> {
		match exec_to_stdout(Some(&self.dir), "kubectl", &["config", "delete-context", ctx], false) {
			Ok(_) => Ok(()),
			Err(e) => Err(KddError::KubectlFail(e.to_string())),
		}
	}

	pub fn k_current_context(&self) -> Result<String, KddError> {
		match exec_to_stdout(Some(&self.dir), "kubectl", &["config", "current-context"], false) {
			Ok(name) => Ok(name.trim().to_string()),
			Err(ex) => Err(KddError::KubectlFail(ex.to_string())),
		}
	}

	pub fn k_set_context(&self, ctx: &str) -> Result<(), KddError> {
		match exec_to_stdout(Some(&self.dir), "kubectl", &["config", "use-context", ctx], false) {
			Ok(_) => Ok(()),
			Err(ex) => Err(KddError::FailSetRealm(ex.to_string())),
		}
	}
}
