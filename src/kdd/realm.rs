////////////////////////////////////
// kdd::realm - All realm related actions
////

use super::{
	error::KddError,
	provider::{AwsProvider, CommonProvider, GcpProvider, Provider, RealmProvider},
	Kdd,
};
use crate::utils::yamls::{as_bool, as_string, as_strings, to_string};
use std::{
	collections::{HashMap, HashSet},
	fs::read_dir,
	io::stdin,
	path::PathBuf,
};
use yaml_rust::Yaml;

const REALM_KEY_YAML_DIR: &str = "yaml_dir";
const REALM_KEY_CONTEXT: &str = "context";
const REALM_KEY_CONFIRM_DELETE: &str = "confirm_delete";
const REALM_KEY_PROJECT: &str = "project"; // for GKE
const REALM_KEY_REGISTRY: &str = "registry"; // must on AWS (inferred for gke and docker-dekstop)
const REALM_KEY_PROFILE: &str = "profile"; // for AWS
const REALM_KEY_CONFIGURATIONS: &str = "default_configurations"; // for AWS

//// Realm Struct
#[derive(Debug)]
pub struct Realm {
	pub name: String,
	pub confirm_delete: bool,
	pub vars: HashMap<String, String>,
	pub registry: Option<String>,
	pub profile: Option<String>,
	pub project: Option<String>,
	pub default_configurations: Option<Vec<String>>,
	provider: RealmProvider,
	yaml_dirs: Vec<PathBuf>,
	context: Option<String>,
}

//// Realm Public Methods
impl Realm {
	pub fn provider_from_ctx(ctx: &str) -> Result<RealmProvider, KddError> {
		if ctx.starts_with("arn:aws") {
			Ok(RealmProvider::Aws(AwsProvider))
		} else if ctx.starts_with("gke") {
			Ok(RealmProvider::Gcp(GcpProvider))
		} else {
			Ok(RealmProvider::Common(CommonProvider))
		}
	}

	pub fn provider(&self) -> &dyn Provider {
		match &self.provider {
			RealmProvider::Aws(p) => p as &dyn Provider,
			RealmProvider::Gcp(p) => p as &dyn Provider,
			RealmProvider::Common(p) => p as &dyn Provider,
		}
	}

	pub fn is_local_registry(&self) -> bool {
		// return true if no registry or registry is localhost or 127.0.0.1
		self.registry
			.as_ref()
			.map(|registry| registry.contains("localhost") || registry.contains("127.0.0.1"))
			.unwrap_or(true)
	}

	pub fn profile(&self) -> String {
		self.profile.as_deref().unwrap_or("default").to_string()
	}

	pub fn k8s_files(&self, names: Option<&[&str]>) -> Vec<PathBuf> {
		let mut yaml_paths: Vec<PathBuf> = Vec::new();

		// if we have a names above
		if let Some(names) = names {
			for name in names.iter() {
				let mut yaml_dirs = self.yaml_dirs.iter();
				loop {
					match yaml_dirs.next() {
						Some(dir_path) => {
							let yaml_path = dir_path.join(format!("{}.yaml", name));
							if yaml_path.is_file() {
								yaml_paths.push(yaml_path);
								break;
							}
						}
						None => break,
					}
				}
			}
		}
		// otherwise, get all of the file (first  file_stem wins)
		else {
			let mut stems_set: HashSet<String> = HashSet::new();

			for yaml_dir in &self.yaml_dirs {
				if let Ok(paths) = read_dir(yaml_dir) {
					for path in paths {
						if let Ok(path) = path {
							let path = path.path();
							if let (Some(stem), Some(ext)) =
								(path.file_stem().map(|v| v.to_str()).flatten(), path.extension().map(|v| v.to_str()).flatten())
							{
								if path.is_file() && ext.to_lowercase() == "yaml" && !stems_set.contains(stem) {
									stems_set.insert(stem.to_string());
									yaml_paths.push(path);
								}
							}
						}
					}
				}
			}
		}

		// Note: Should be ok to be lossy here
		yaml_paths.sort_by_key(|v| v.file_stem().unwrap().to_string_lossy().to_string());

		yaml_paths
	}

	pub fn k8s_out_dir(&self) -> PathBuf {
		// Note: There is always at least one yaml_dir, per parse_realm logic
		let yaml_dir = &self.yaml_dirs[0];
		yaml_dir.join(".out/").join(&self.name)
	}

	pub fn default_configurations(&self) -> Option<Vec<&str>> {
		let names = self
			.default_configurations
			.as_ref()
			.map(|v| v.iter().map(|v| v as &str).collect::<Vec<&str>>());

		names
	}
}

//// Realm Builder(s)
impl Realm {
	pub fn from_yaml(kdd_dir: &PathBuf, name: &str, yaml: &Yaml) -> Result<Realm, KddError> {
		let ctx = as_string(yaml, REALM_KEY_CONTEXT).ok_or_else(|| KddError::MissingRealmContext(name.to_string()))?;
		let provider = Realm::provider_from_ctx(&ctx)?;

		// get the string or strings values as an array of string
		let yaml_dirs = as_string(yaml, REALM_KEY_YAML_DIR)
			.map(|v| vec![v.to_string()])
			.or_else(|| as_strings(yaml, REALM_KEY_YAML_DIR))
			.unwrap_or_else(|| Vec::new());

		if yaml_dirs.len() == 0 {
			return Err(KddError::FailLoadNoK8sYamlDir(name.to_string()));
		}

		// create the pathbuff
		let yaml_dirs: Vec<PathBuf> = yaml_dirs.into_iter().map(|v| kdd_dir.join(v)).collect();

		// extract the eventual confirm_delete and then delete it
		let confirm_delete = as_bool(yaml, REALM_KEY_CONFIRM_DELETE).unwrap_or(true);

		let mut exclude_vars = HashSet::new();
		exclude_vars.insert(REALM_KEY_CONFIRM_DELETE);

		let mut vars: HashMap<String, String> = HashMap::new();
		// add all of the root variables as vars
		if let Some(map) = yaml.as_hash() {
			for (name, val) in map.iter() {
				if let (Some(name), Some(val)) = (name.as_str(), to_string(val)) {
					if !exclude_vars.contains(name) {
						vars.insert(name.to_owned(), val);
					}
				}
			}
		}

		Ok(Realm {
			name: name.to_string(),
			confirm_delete,
			vars,
			provider: provider,
			yaml_dirs,
			context: as_string(yaml, REALM_KEY_CONTEXT),
			project: as_string(yaml, REALM_KEY_PROJECT),
			registry: as_string(yaml, REALM_KEY_REGISTRY),
			profile: as_string(yaml, REALM_KEY_PROFILE),
			default_configurations: as_strings(yaml, REALM_KEY_CONFIGURATIONS),
		})
	}
}

//// Kdd Realm Methods
impl Kdd {
	pub fn realm_for_ctx(&self, ctx: &str) -> Option<&Realm> {
		self.realms()
			.into_iter()
			.find(|v| v.context.as_deref().map(|vc| vc == ctx).unwrap_or(false))
	}

	pub fn current_realm(&self) -> Result<Option<&Realm>, KddError> {
		let ctx = self.k_current_context()?;
		// TODO: Set the project if realm found
		Ok(self.realm_for_ctx(&ctx))
	}

	pub fn realms(&self) -> Vec<&Realm> {
		self.realms.values().collect()
	}

	pub fn realm_set(&self, name: &str) -> Result<(), KddError> {
		match self.realms.get(name) {
			None => Err(KddError::RealmNotFound(name.to_string())),
			Some(realm) => {
				match &realm.context {
					Some(ctx) => {
						let ctxs = self.k_list_context()?;
						let ctxs_set: HashSet<_> = HashSet::from_iter(ctxs);

						if !ctxs_set.contains(ctx) {
							println!("Kubernetes context {} does not exist. Do you want to create it and set it? (YES to continue, anything else to cancel)", ctx);
							let mut guess = String::new();
							stdin().read_line(&mut guess).expect("Failed to read line");

							if guess.trim() != "YES" {
								println!("Canceling kubernetes context creation");
								return Ok(());
							}
							self.k_create_context(&ctx);
							self.k_set_context(&ctx);
						} else {
							self.k_set_context(&ctx);
						}

						Ok(())
					}
					None => Err(KddError::RealmHasNoContext(name.to_string())),
				}
			}
		}
	}

	pub fn print_realms(&self) -> Result<(), KddError> {
		let current_realm = self.current_realm()?;
		let current_ctx = current_realm.map(|r| r.context.as_deref()).flatten();
		let realms = self.realms();
		tr_print(false, "REALM", "TYPE", "PROFILE/PROJECT", "CONTEXT");

		for realm in realms {
			let pr = realm.profile.as_deref().or(realm.project.as_deref()).unwrap_or("-");
			let ctx = realm.context.as_deref();
			let typ = realm.provider.to_string();
			let is_current = ctx.is_some() && current_ctx == ctx;
			let ctx = ctx.unwrap_or("-");
			tr_print(is_current, &realm.name, &typ, pr, ctx);
		}

		Ok(())
	}
}

// region:    Utils
fn tr_print(sel: bool, realm: &str, typ: &str, prj: &str, ctx: &str) {
	let sel = if sel {
		"*"
	} else {
		" "
	};
	println!("{}  {: <12}{: <14}{: <20}{}", sel, realm, typ, prj, ctx);
}
// endregion: Utils
