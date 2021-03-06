use std::collections::HashSet;

use crate::utils::{exec_cmd_args, exec_to_stdout};

use super::{error::KddError, realm::Realm, Kdd, Pod};

impl Kdd {
	pub fn k_exec(&self, _realm: &Realm, names: Option<&[&str]>, pod_args: &[&str]) -> Result<(), KddError> {
		let mut pods = Self::k_list_pods()?;

		//// filter by names names
		if let Some(names) = names {
			let names_set: HashSet<String> = names.iter().map(|v| format!("{}-{}", self.system, v)).collect();
			pods = pods.into_iter().filter(|pod| names_set.contains(&pod.service_name)).collect();
		}

		kexec_pods(&pods, pod_args)?;

		Ok(())
	}
}

fn kexec_pods(pods: &Vec<Pod>, pod_args: &[&str]) -> Result<(), KddError> {
	for pod in pods.iter() {
		let mut args: Vec<&str> = vec!["exec", &pod.name, "--"];
		args.extend_from_slice(pod_args);

		exec_cmd_args(None, "kubectl", &args)?;
	}

	Ok(())
}
