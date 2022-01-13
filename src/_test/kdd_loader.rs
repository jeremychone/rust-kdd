use std::error::Error;

use crate::test_utils::*;

const APP_1_BLOCK_NAMES: [&str; 12] = [
	"db",
	"queue",
	"mock-s3",
	"agent2",
	"_common",
	"vid-scaler",
	"vid-init",
	"agent",
	"web",
	"web-server",
	"admin",
	"admin-server",
];
const APP_1_REALM_NAMES: [&str; 3] = ["dev", "aws", "app-prod"];
const APP_1_BUILDER_NAMES: [&str; 4] = ["npm_install", "tsc", "rollup", "pcss"];

#[test]
fn loader_structure() -> Result<(), Box<dyn Error>> {
	let kdd = load_kdd()?;

	//// Check structure
	let realm_names: Vec<&str> = kdd.realms().iter().map(|r| &r.name as &str).collect();
	assert_eq!(APP_1_REALM_NAMES.to_vec(), realm_names);

	let block_names: Vec<&str> = kdd.blocks.iter().map(|b| &b.name as &str).collect();
	assert_eq!(APP_1_BLOCK_NAMES.to_vec(), block_names);

	let builder_names: Vec<&str> = kdd.builders.iter().map(|b| &b.name as &str).collect();
	assert_eq!(APP_1_BUILDER_NAMES.to_vec(), builder_names);

	Ok(())
}

#[test]
fn loader_dev_realm() -> Result<(), Box<dyn Error>> {
	let kdd = load_kdd()?;

	// CHECK - dev realm
	let realm = &kdd.realms["dev"];
	let vars = &realm.vars;
	// normal dev realm property
	assert_eq!(Some("Some dev stuff"), get_str(vars, "dev_stuff"), "dev_stuff realm var");
	// property inherited from _base_
	assert_eq!(Some("8080"), get_str(vars, "ext_port"), "ext_port realm var");
	// property overriden by dev
	assert_eq!(Some("4"), get_str(vars, "web_server_replicas"), "ext_port realm var");

	assert_eq!(false, realm.confirm_delete);

	Ok(())
}

#[test]
fn loader_prod_realm() -> Result<(), Box<dyn Error>> {
	let kdd = load_kdd()?;

	//// Check app-prod realm
	let realm = &kdd.realms["app-prod"];
	let vars = &realm.vars;
	// prod_stuff property should be None
	assert_eq!(Some("Some prod stuff"), get_str(vars, "prod_stuff"), "prod_stuff realm var");
	// dev_stuff property should be None
	assert_eq!(None, get_str(vars, "dev_stuff"), "dev_stuff realm var");
	// came from the _base_
	assert_eq!(Some("8080"), get_str(vars, "ext_port"), "ext_port realm var");
	// by default should be true
	assert_eq!(true, realm.confirm_delete);

	Ok(())
}
