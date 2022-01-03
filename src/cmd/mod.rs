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
			if let Some((_, sub)) = &app.subcommand() {
				sub.value_of("root_dir")
			} else {
				None
			}
		})
		.unwrap_or("./");

	match app.subcommand() {
		Some(("build", sub_cmd)) => exec_build(root_dir, sub_cmd, false)?,
		Some(("watch", sub_cmd)) => exec_watch(root_dir, sub_cmd)?,
		Some(("dbuild", sub_cmd)) => exec_build(root_dir, sub_cmd, true)?,
		Some(("dpush", sub_cmd)) => exec_dpush(root_dir, sub_cmd)?,
		Some(("realm", sub_cmd)) => exec_realm(root_dir, sub_cmd)?,
		Some(("ktemplate", sub_cmd)) => exec_kaction("template", root_dir, sub_cmd)?,
		Some(("kapply", sub_cmd)) => exec_kaction("apply", root_dir, sub_cmd)?,
		Some(("kcreate", sub_cmd)) => exec_kaction("create", root_dir, sub_cmd)?,
		Some(("kdelete", sub_cmd)) => exec_kaction("delete", root_dir, sub_cmd)?,
		Some(("klog", sub_cmd)) => exec_klog(root_dir, sub_cmd)?,
		Some(("kexec", sub_cmd)) => exec_kexec(root_dir, sub_cmd)?,
		Some(("version", sub_cmd)) => exec_version(root_dir, sub_cmd)?,
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

fn exec_watch(root_dir: &str, argc: &ArgMatches) -> Result<(), AppError> {
	let kdd = load_kdd(root_dir)?;
	let blocks = argc.value_of("blocks").map(|v| v.split(",").into_iter().collect::<Vec<&str>>());
	let blocks = blocks.as_ref().map(|v| &v[..]);

	kdd.watch(blocks)?;

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

fn exec_kaction(action: &str, root_dir: &str, argc: &ArgMatches) -> Result<(), AppError> {
	let kdd = load_kdd(root_dir)?;
	let realm = kdd.current_realm()?;
	let names = split_names(argc.value_of("names"));
	let names = names.as_ref().map(|v| &v[..]);

	if let Some(realm) = realm {
		// if no names, get the names from the realm default_configurations if present
		let config_names = realm.default_configurations();
		let names = names.or_else(|| config_names.as_ref().map(|v| v.as_slice()));

		match action {
			"apply" => kdd.k_apply(realm, names)?,
			"create" => kdd.k_create(realm, names)?,
			"delete" => kdd.k_delete(realm, names)?,
			"template" => kdd.k_templates(realm, names, true).map(|result| ())?,
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

fn exec_kexec(root_dir: &str, argc: &ArgMatches) -> Result<(), AppError> {
	let kdd = load_kdd(root_dir)?;
	let realm = kdd.current_realm()?;

	if let Some(realm) = realm {
		let names = split_names(argc.value_of("names"));
		let names = names.as_ref().map(|v| &v[..]);

		// the pod args (which might contain the cmd for the pod)
		let pod_args = argc
			.values_of("pod_args")
			.map(|v| v.into_iter().map(|d| d.to_string()).collect::<Vec<String>>());

		if let Some(pod_args) = pod_args {
			// if -b, then, we add the /bin/bash -c, and the pod_args become one cmd component
			let pod_args = if argc.is_present("bash") {
				vec!["/bin/bash".to_string(), "-c".to_string(), pod_args.join(" ")]
			} else {
				pod_args
			};

			// make it Vec<&str>
			let pod_args: Vec<&str> = pod_args.iter().map(|v| v as &str).collect();
			// call the exec
			kdd.k_exec(realm, names, &pod_args[..])?;
		} else {
			println!("Cannot run kubectl exec , no sub command. Use `kdd kexec -- /bin/bash -c 'ls'` for example");
		}
	} else {
		println!("Cannot run kubectl exec, no current realm");
	}

	Ok(())
}

fn exec_version(root_dir: &str, _: &ArgMatches) -> Result<(), AppError> {
	let kdd = load_kdd(root_dir)?;

	kdd.version(&mut std::io::stdout())?;

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
