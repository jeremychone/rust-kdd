////////////////////////////////////
// cmd - Module to execute clap commands
////

use self::argc::cmd_app;
use crate::{
	app_error::AppError,
	kdd::{error::KddError, Kdd},
};
use clap::ArgMatches;
use std::path::Path;

mod argc;

pub fn cmd_run() -> Result<(), AppError> {
	let app = cmd_app().get_matches();
	let root_dir = app
		.value_of("root_dir")
		.or_else(|| {
			if let (_, Some(sub)) = &app.subcommand() {
				sub.value_of("root_dir")
			} else {
				None
			}
		})
		.unwrap_or("./");

	match app.subcommand() {
		("build", Some(sub_cmd)) => exec_build(root_dir, sub_cmd, false)?,
		("dbuild", Some(sub_cmd)) => exec_build(root_dir, sub_cmd, true)?,
		("dpush", Some(sub_cmd)) => exec_dpush(root_dir, sub_cmd)?,
		("realm", Some(sub_cmd)) => exec_realm(root_dir, sub_cmd)?,
		("ktemplate", Some(sub_cmd)) => exec_ktemplate(root_dir, sub_cmd)?,
		("kapply", Some(sub_cmd)) => exec_kaction("apply", root_dir, sub_cmd)?,
		("kcreate", Some(sub_cmd)) => exec_kaction("create", root_dir, sub_cmd)?,
		("kdelete", Some(sub_cmd)) => exec_kaction("delete", root_dir, sub_cmd)?,
		("klog", Some(sub_cmd)) => exec_klog(root_dir, sub_cmd)?,
		_ => {
			// needs cmd_app version as the orginal got consumed by get_matches
			cmd_app().print_long_help()?;
			println!("\n");
		}
	}

	Ok(())
}

// region:    Command Execs
fn exec_build(root_dir: &str, argc: &ArgMatches, docker_build: bool) -> Result<(), AppError> {
	let kdd = load_kdd(root_dir)?;

	let blocks = argc.value_of("blocks").map(|v| v.split(",").into_iter().collect::<Vec<&str>>());
	let blocks = blocks.as_ref().map(|v| &v[..]);

	kdd.build(blocks, docker_build)?;

	Ok(())
}

fn exec_dpush(root_dir: &str, argc: &ArgMatches) -> Result<(), AppError> {
	let kdd = load_kdd(root_dir)?;

	let blocks = split_names(argc.value_of("blocks"));
	let blocks = blocks.as_ref().map(|v| &v[..]);

	let realm = kdd.current_realm()?.ok_or_else(|| KddError::DpushFailNoRealm)?;

	kdd.d_push(realm, blocks)?;

	Ok(())
}

fn exec_realm(root_dir: &str, argc: &ArgMatches) -> Result<(), AppError> {
	let kdd = load_kdd(root_dir)?;

	if let Some(name) = argc.value_of("name") {
		println!("Change realm to {}", name);
		kdd.realm_set(name)?;
		kdd.print_realms()?;
	}
	// if no name, we list the realms
	else {
		kdd.print_realms()?;
	}

	Ok(())
}

fn exec_ktemplate(root_dir: &str, argc: &ArgMatches) -> Result<(), AppError> {
	let kdd = load_kdd(root_dir)?;
	let realm = kdd.current_realm()?;

	let names = split_names(argc.value_of("names"));
	let names = names.as_ref().map(|v| &v[..]);

	if let Some(realm) = realm {
		kdd.k_templates(realm, names, true)?;
	} else {
		println!("Cannot run ktemplate, no current realm");
	}

	Ok(())
}

fn exec_kaction(action: &str, root_dir: &str, argc: &ArgMatches) -> Result<(), AppError> {
	let kdd = load_kdd(root_dir)?;
	let realm = kdd.current_realm()?;
	let names = split_names(argc.value_of("names"));
	let names = names.as_ref().map(|v| &v[..]);
	if let Some(realm) = realm {
		match action {
			"apply" => kdd.k_apply(realm, names)?,
			"create" => kdd.k_create(realm, names)?,
			"delete" => kdd.k_delete(realm, names)?,
			_ => (),
		}
	} else {
		println!("Cannot run '{}', no current realm", action);
	}

	Ok(())
}

fn exec_klog(root_dir: &str, argc: &ArgMatches) -> Result<(), AppError> {
	let kdd = load_kdd(root_dir)?;
	let realm = kdd.current_realm()?;

	let names = split_names(argc.value_of("names"));
	let names = names.as_ref().map(|v| &v[..]);

	if let Some(realm) = realm {
		kdd.k_log(realm, names)?;
	} else {
		println!("Cannot run kubectl log, no current realm");
	}

	Ok(())
}

// endregion: Command Execs

// region:    Utils

/// Split a opt str by ","
fn split_names(val: Option<&str>) -> Option<Vec<&str>> {
	val.map(|v| v.split(",").into_iter().collect::<Vec<&str>>())
}

fn load_kdd<'a>(root_dir: &str) -> Result<Kdd<'a>, AppError> {
	let dir = Path::new(root_dir).to_path_buf();
	Ok(Kdd::from_dir(dir)?)
}

// endregion: Utils
