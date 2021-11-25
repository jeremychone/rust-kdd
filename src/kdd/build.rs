use std::{
	cell::RefCell,
	collections::{HashMap, HashSet},
	time::Duration,
};

use futures::future::join_all;
use tokio::time::sleep;

use super::{builder::Builder, error::KddError, Block, Kdd};

impl<'a> Kdd<'a> {
	pub fn blocks_for_names(&self, names: Option<&[&str]>, docker_block: bool) -> Result<(Vec<&Block>, HashMap<&str, &Block>), KddError> {
		let block_by_name: HashMap<&str, &Block> = self.blocks.iter().map(|b| (b.name.as_str(), b)).collect();

		let mut blocks_to_build = match names {
			Some(names) => {
				let mut blocks: Vec<&Block> = Vec::new();
				for name in names {
					match block_by_name.get(name) {
						Some(block) => blocks.push(block),
						None => return Err(KddError::BlockUnknown(name.to_string())),
					}
				}
				blocks
			}
			None => self.blocks.iter().collect::<Vec<&Block>>(),
		};

		// if dbuild, make sure all docker file
		if docker_block {
			blocks_to_build = blocks_to_build
				.into_iter()
				.filter(|b| self.get_block_dir(b).join("Dockerfile").is_file())
				.collect();
		}

		Ok((blocks_to_build, block_by_name))
	}

	fn builders_for_block(&self, block: &Block) -> Vec<&Builder> {
		let mut block_builders: Vec<&Builder> = Vec::new();
		let mut replace_names: HashSet<&str> = HashSet::new();
		for builder in self.builders.iter() {
			if let Some(when_file) = &builder.when_file {
				let when_path = self.get_rel_path(block, when_file);
				if when_path.is_file() {
					block_builders.push(builder);
					if let Some(replace) = &builder.replace {
						replace_names.insert(replace);
					}
				}
			}
		}

		let block_builders = block_builders
			.into_iter()
			.filter(|b| !replace_names.contains(b.name.as_str()))
			.collect();

		block_builders
	}

	#[tokio::main(flavor = "current_thread")]
	pub async fn watch(&self, names: Option<&[&str]>) -> Result<(), KddError> {
		let (blocks_to_build, _) = self.blocks_for_names(names, false)?;

		let mut handles = vec![];

		for block in blocks_to_build.iter() {
			for builder in self.builders_for_block(block).iter() {
				let block_dir = self.get_block_dir(&block);
				let kdd_dir = self.dir.clone();
				let exec = builder.exec.clone();

				handles.push(tokio::spawn(async move {
					let _ = exec.execute_and_wait(kdd_dir.as_path(), block_dir.as_path(), true).await;
				}));

				// give some time for each builder to get started (better console readability)
				sleep(Duration::from_secs(2)).await;
			}
		}

		join_all(handles).await;

		Ok(())
	}

	pub fn build(&self, names: Option<&[&str]>, docker_build: bool) -> Result<(), KddError> {
		let (blocks_to_build, block_by_name) = self.blocks_for_names(names, docker_build)?;

		// we get the current realm to the automatic dpush when local (desktop)
		let current_realm = &self.current_realm().ok().flatten();
		// if realm desktop start with true (can be set to false later if fail at first time)
		let mut push_to_local_registry = match current_realm {
			Some(realm) => realm.is_desktop(),
			_ => false,
		};

		// RefCell enough since single thread (need to put Arc/Mutex if async)
		let blocks_built: RefCell<HashSet<String>> = RefCell::new(HashSet::new());

		// create the non mutable version
		let blocks_to_build = blocks_to_build;

		let build_block = |block: &Block| {
			let block_dir = self.get_block_dir(&block);
			let mut has_builder = false;

			for builder in self.builders_for_block(block).iter() {
				if !has_builder {
					has_builder = true;
					println!("===  Executing Builders for '{}' ", block.name);
				}
				println!("--- builder - {} for [{}]", builder.name, block.name);
				// ignore error
				let _ = builder.exec.execute_and_wait(&self.dir, &block_dir, false);
				println!();
			}
			blocks_built.borrow_mut().insert(block.name.to_string());
			if has_builder {
				println!("====== /Executing Builders for '{}' DONE", block.name);
			}
		};

		for block in blocks_to_build {
			println!("==================   Block '{}' building... ==================", block.name);
			if let Some(dependencies) = &block.dependencies {
				for block_name in dependencies.iter() {
					if blocks_built.borrow().contains(block_name) {
						println!("Dependency {} already built, skipping", block_name);
					} else {
						match block_by_name.get(block_name.as_str()) {
							Some(dep_block) => {
								println!("======  Dependency '{}' for '{}' building... ", dep_block.name, block.name);
								build_block(dep_block);
								println!("====== /Dependency '{}' for '{}' DONE\n", dep_block.name, block.name);
							}
							None => {
								println!("Dependency {} not a block name, skipping. \n{:?}", block_name, block);
							}
						}
					}
				}
			}
			build_block(block);

			if docker_build {
				println!("======  Docker Build for '{}' ", block.name);
				self.d_build_block(block)?;
				println!("====== /Docker Build for '{}' DONE ", block.name);

				if push_to_local_registry {
					if let Some(realm) = current_realm {
						match self.d_push(realm, Some(&[block.name.as_ref()])) {
							Ok(_) => (),
							Err(ex) => {
								println!("WARNING dpush to local registry failed. Cause: {}", ex);
								println!("Skip dpush to local registry from now on.");
								push_to_local_registry = false;
							}
						}
					}
				}
			}

			println!("==================  /Block '{}' DONE  ==================", block.name);
			println!();
		}

		Ok(())
	}
}
