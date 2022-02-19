use clap::{crate_version, Arg, Command};

pub fn cmd_app() -> Command<'static> {
	Command::new("kdd")
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
fn sub_build() -> Command<'static> {
	Command::new("build")
		.about("Build one or more block")
		.arg(Arg::new("blocks").help("Comma delimited block names (no space)"))
		.arg(arg_root_dir())
}

fn sub_watch() -> Command<'static> {
	Command::new("watch")
		.about("Watch one or more block")
		.arg(Arg::new("blocks").help("Comma delimited block names (no space)"))
		.arg(arg_root_dir())
}

fn sub_dbuild() -> Command<'static> {
	Command::new("dbuild")
		.about("Build and docker build one or more block")
		.arg(Arg::new("blocks").help("Comma delimited block names (no space)"))
		.arg(arg_root_dir())
}

fn sub_dpush() -> Command<'static> {
	Command::new("dpush")
		.about("Docker push one or more block to a realm")
		.arg(Arg::new("blocks").help("Comma delimited block names (no space)"))
		.arg(arg_root_dir())
}

fn sub_realm() -> Command<'static> {
	Command::new("realm")
		.about("Show the available realms or set the current realm")
		.arg(Arg::new("name").help("Realm name to change to"))
		.arg(arg_root_dir())
}

fn sub_kaction(action_n_alias: (&'static str, &'static str)) -> Command<'static> {
	let (action, alias) = action_n_alias;
	// with format!(...)
	let about = match action {
		"kapply" => "Excute kubectl apply for one or more kubernetes configuration file",
		"kcreate" => "Excute kubectl create for one or more kubernetes configuration file",
		"kdelete" => "Excute kubectl delete for one or more kubernetes configuration file",
		_ => "not supported",
	};
	Command::new(action)
		.about(about)
		.alias(alias)
		.arg(
			Arg::new("names")
				.help("Yaml file name or comma delimited names (no space, without path or extension, .e.g, web-server for k8s/dev/web-server.yaml"),
		)
		.arg(arg_root_dir())
}

fn sub_ktemplate() -> Command<'static> {
	Command::new("ktemplate")
		.about("Render the k8s yaml files for the current realm")
		.arg(
			Arg::new("names")
				.help("Yaml file name or comma delimited names (no space, without path or extension, .e.g, web-server for k8s/dev/web-server.yaml"),
		)
		.arg(arg_root_dir())
}

fn sub_klog() -> Command<'static> {
	Command::new("klog")
		.alias("klogs")
		.alias("kl")
		.about("Execute and display the kubectl log for one or more service (and all of their respective pods)")
		.arg(
			Arg::new("names")
				.help("Service names that match the pod label run: system-_service_name_ (e.g., 'web-server' for label.run = cstar-web-server)"),
		)
		.arg(arg_root_dir())
}

fn sub_kexec() -> Command<'static> {
	Command::new("kexec")
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

fn sub_version() -> Command<'static> {
	Command::new("version")
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
