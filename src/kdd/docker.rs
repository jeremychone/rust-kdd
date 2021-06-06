////////////////////////////////////
// kdd::docker - handle all docker actions
////

use super::{error::KddError, Block, Kdd, Realm};
use crate::utils::exec_cmd_args;

impl<'a> Kdd<'a> {
	// e.g., docker build --rm -t localhost:5000/cstar-db:DROP-002-SNAPSHOT .
	pub fn d_build_block(&self, block: &Block) -> Result<(), KddError> {
		let cwd = self.get_block_dir(&block);

		let image_uri = &self.image_uri(block, None);

		// exec command
		let args = &["build", "--rm", "-t", &image_uri, "."];

		match exec_cmd_args(Some(&cwd), "docker", args) {
			Ok(_) => Ok(()),
			Err(ex) => Err(KddError::FailDockerBuilder(ex.to_string())),
		}
	}

	pub fn d_push(&self, realm: &Realm, names: Option<&[&str]>) -> Result<(), KddError> {
		let (blocks, _) = self.blocks_for_names(names, true)?;

		realm.provider().before_dpushes(&self.system, realm, &blocks)?;

		for block in blocks {
			self.d_push_block(realm, block)?;
		}

		Ok(())
	}

	pub fn d_push_block(&self, realm: &Realm, block: &Block) -> Result<(), KddError> {
		let cwd = &self.dir;

		let local_image_uri = &self.image_uri(block, None);
		let remote_image_uri = &self.image_uri(block, Some(realm));

		println!("======  Pushing image {} : {}", local_image_uri, remote_image_uri);
		// make sure the tags exist
		exec_cmd_args(Some(cwd), "docker", &["tag", local_image_uri, remote_image_uri])?;

		match exec_cmd_args(Some(cwd), "docker", &["push", remote_image_uri]) {
			Ok(_) => (),
			Err(ex) => {
				println!("Failed to do a docker push (cause: {}). Trying to recover...", ex);
				realm.provider().docker_auth(realm)?;
				match exec_cmd_args(Some(cwd), "docker", &["push", remote_image_uri]) {
					Ok(_) => {
						println!("Recovered OK!");
					}
					Err(ex) => return Err(KddError::DpushFailed(ex.to_string())),
				}
			}
		}

		//
		println!("====== /Pushing image {} : {} - DONE\n", local_image_uri, remote_image_uri);

		Ok(())
	}

	fn image_uri(&self, block: &Block, realm: Option<&Realm>) -> String {
		let registry = realm.map(|r| r.registry.as_deref()).flatten().unwrap_or("localhost:5000");
		let registry = registry.trim_end_matches('/');
		let image_name = self.image_name(block);

		format!("{}/{}:{}", registry, &image_name, self.image_tag())
	}
}
