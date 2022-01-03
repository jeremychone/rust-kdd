use clap::{crate_version, App, Arg};

pub fn cmd_app() -> App<'static> {
	App::new("kdd")
		.version(&crate_version!()[..])
		.about("Kubernetes Driven Development and Deployment")
		.arg(arg_root_dir())
		.subcommand(sub_build())
		.subcommand(sub_watch())
		.subcommand(sub_dbuild())
		.subcommand(sub_dpush())
		.subcommand(sub_realm())
		.subcommand(sub_ktemplate())
		.subcommand(sub_kaction(("kapply", "ka")))
		.subcommand(sub_kaction(("kcreate", "kc")))
		.subcommand(sub_kaction(("kdelete", "kd")))
		.subcommand(sub_klog())
		.subcommand(sub_kexec())
		.subcommand(sub_version())
}

// region:    Subcommands
fn sub_build() -> App<'static> {
	App::new("build")
		.about("Build one or more block")
		.arg(Arg::new("blocks").help("Comma delimited block names (no space)"))
		.arg(arg_root_dir())
}

fn sub_watch() -> App<'static> {
	App::new("watch")
		.about("Watch one or more block")
		.arg(Arg::new("blocks").help("Comma delimited block names (no space)"))
		.arg(arg_root_dir())
}

fn sub_dbuild() -> App<'static> {
	App::new("dbuild")
		.about("Build and docker build one or more block")
		.arg(Arg::new("blocks").help("Comma delimited block names (no space)"))
		.arg(arg_root_dir())
}

fn sub_dpush() -> App<'static> {
	App::new("dpush")
		.about("Docker push one or more block to a realm")
		.arg(Arg::new("blocks").help("Comma delimited block names (no space)"))
		.arg(arg_root_dir())
}

fn sub_realm() -> App<'static> {
	App::new("realm")
		.about("Show the available realms or set the current realm")
		.arg(Arg::new("name").help("Realm name to change to"))
		.arg(arg_root_dir())
}

fn sub_kaction(action_n_alias: (&'static str, &'static str)) -> App<'static> {
	let (action, alias) = action_n_alias;
	// with format!(...)
	let about = match action {
		"kapply" => "Excute kubectl apply for one or more kubernetes configuration file",
		"kcreate" => "Excute kubectl create for one or more kubernetes configuration file",
		"kdelete" => "Excute kubectl delete for one or more kubernetes configuration file",
		_ => "not supported",
	};
	App::new(action)
		.about(about)
		.alias(alias)
		.arg(
			Arg::new("names")
				.help("Yaml file name or comma delimited names (no space, without path or extension, .e.g, web-server for k8s/dev/web-server.yaml"),
		)
		.arg(arg_root_dir())
}

fn sub_ktemplate() -> App<'static> {
	App::new("ktemplate")
		.about("Render the k8s yaml files for the current realm")
		.arg(
			Arg::new("names")
				.help("Yaml file name or comma delimited names (no space, without path or extension, .e.g, web-server for k8s/dev/web-server.yaml"),
		)
		.arg(arg_root_dir())
}

fn sub_klog() -> App<'static> {
	App::new("klog")
		.alias("klogs")
		.alias("kl")
		.about("Execute and display the kubectl log for one or more service (and all of their respective pods)")
		.arg(
			Arg::new("names")
				.help("Service names that match the pod label run: system-_service_name_ (e.g., 'web-server' for label.run = cstar-web-server)"),
		)
		.arg(arg_root_dir())
}

fn sub_kexec() -> App<'static> {
	App::new("kexec")
		.about("Execute a kubectl exec for all pods matching service_name")
		.alias("kx")
		.arg(arg_root_dir())
		.arg(
			Arg::new("names")
				.help("Service names that match the pod label run: system-_service_name_ (e.g., 'web-server' for label.run = cstar-web-server)"),
		)
		.arg(
			Arg::new("bash")
				.short('b')
				.long("bash")
				.takes_value(false)
				.help("Execute the -- commands inside a /bin/bash -c '...' "),
		)
		.arg(Arg::new("pod_args").multiple_values(true))
}

fn sub_version() -> App<'static> {
	App::new("version")
		.about("Version files with the configured version replace commands")
		.arg(arg_root_dir())
}

// endregion: Subcommands

// region:    Common Args
fn arg_root_dir() -> Arg<'static> {
	Arg::new("root_dir")
		.short('d')
		.takes_value(true)
		.help("The root dir where the driving kdd.yaml reside")
}

// endregion: Common Args
