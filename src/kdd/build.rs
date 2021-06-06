use std::{
	cell::RefCell,
	collections::{HashMap, HashSet},
};

use super::{error::KddError, Block, Kdd};

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

	pub fn build(&self, names: Option<&[&str]>, docker_build: bool) -> Result<(), KddError> {
		let (blocks_to_build, block_by_name) = self.blocks_for_names(names, docker_build)?;

		// RefCell enough since single thread (need to put Arc/Mutex if async)
		let blocks_built: RefCell<HashSet<String>> = RefCell::new(HashSet::new());

		// create the non mutable version
		let blocks_to_build = blocks_to_build;

		let build = |block: &Block| {
			let block_dir = self.get_block_dir(&block);
			let mut has_builder = false;

			for builder in self.builders.iter() {
				if let Some(when_file) = &builder.when_file {
					let when_path = self.get_rel_path(block, when_file);
					if when_path.is_file() {
						if !has_builder {
							has_builder = true;
							println!("======  Executing Builders for '{}' ", block.name);
						}
						println!("--- builder - {} for [{}] - File {} found", builder.name, block.name, when_file);
						builder.exec.execute(&self.dir, &block_dir);
						println!();
					}
				}
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
								build(dep_block);
								println!("====== /Dependency '{}' for '{}' DONE\n", dep_block.name, block.name);
							}
							None => {
								println!("Dependency {} not a block name, skipping. \n{:?}", block_name, block);
							}
						}
					}
				}
			}
			build(block);

			if docker_build {
				println!("======  Docker Build for '{}' ", block.name);

				self.d_build_block(block)?;

				println!("====== /Docker Build for '{}' DONE ", block.name);
			}

			println!("==================  /Block '{}' DONE  ==================", block.name);
			println!();
		}

		Ok(())
	}
}
