use clap::{crate_version, App, Arg, SubCommand};

pub fn cmd_app() -> App<'static, 'static> {
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
		.subcommand(sub_kaction("kapply"))
		.subcommand(sub_kaction("kcreate"))
		.subcommand(sub_kaction("kdelete"))
		.subcommand(sub_klog())
		.subcommand(sub_kexec())
}

// region:    Subcommands
fn sub_build() -> App<'static, 'static> {
	SubCommand::with_name("build")
		.about("Build one or more block")
		.arg(Arg::with_name("blocks").help("Comma delimited block names (no space)"))
		.arg(arg_root_dir())
}

fn sub_watch() -> App<'static, 'static> {
	SubCommand::with_name("watch")
		.about("Watch one or more block")
		.arg(Arg::with_name("blocks").help("Comma delimited block names (no space)"))
		.arg(arg_root_dir())
}

fn sub_dbuild() -> App<'static, 'static> {
	SubCommand::with_name("dbuild")
		.about("Build and docker build one or more block")
		.arg(Arg::with_name("blocks").help("Comma delimited block names (no space)"))
		.arg(arg_root_dir())
}

fn sub_dpush() -> App<'static, 'static> {
	SubCommand::with_name("dpush")
		.about("Docker push one or more block to a realm")
		.arg(Arg::with_name("blocks").help("Comma delimited block names (no space)"))
		.arg(arg_root_dir())
}

fn sub_realm() -> App<'static, 'static> {
	SubCommand::with_name("realm")
		.about("Show the available realms or set the current realm")
		.arg(Arg::with_name("name").help("Realm name to change to"))
		.arg(arg_root_dir())
}

fn sub_kaction(action: &str) -> App<'static, 'static> {
	// with format!(...)
	let about = match action {
		"kapply" => "Excute kubectl apply for one or more kubernetes configuration file",
		"kcreate" => "Excute kubectl create for one or more kubernetes configuration file",
		"kdelete" => "Excute kubectl delete for one or more kubernetes configuration file",
		_ => "not supported",
	};
	SubCommand::with_name(action)
		.about(about)
		.arg(
			Arg::with_name("names")
				.help("Yaml file name or comma delimited names (no space, without path or extension, .e.g, web-server for k8s/dev/web-server.yaml"),
		)
		.arg(arg_root_dir())
}

fn sub_ktemplate() -> App<'static, 'static> {
	SubCommand::with_name("ktemplate")
		.about("Render the k8s yaml files for the current realm")
		.arg(
			Arg::with_name("names")
				.help("Yaml file name or comma delimited names (no space, without path or extension, .e.g, web-server for k8s/dev/web-server.yaml"),
		)
		.arg(arg_root_dir())
}

fn sub_klog() -> App<'static, 'static> {
	SubCommand::with_name("klog")
		.alias("klogs")
		.about("Execute and display the kubectl log for one or more service (and all of their respective pods)")
		.arg(
			Arg::with_name("names").help("Service names that match the pod label run: system-_service_name_ (e.g., 'web-server' for label.run = cstar-web-server)"),
		)
		.arg(arg_root_dir())
}

fn sub_kexec() -> App<'static, 'static> {
	SubCommand::with_name("kexec")
		.about("Execute a kubectl exec for all pods matching service_name")
		.arg(arg_root_dir())
		.arg(
			Arg::with_name("names").help("Service names that match the pod label run: system-_service_name_ (e.g., 'web-server' for label.run = cstar-web-server)"),
		)
		.arg(
			Arg::with_name("bash")
				.short("b")
				.long("bash")
				.takes_value(false)
				.help("Execute the -- commands inside a /bin/bash -c '...' "),
		)
		.arg(Arg::with_name("pod_args").multiple(true))
}

// endregion: Subcommands

// region:    Common Args
fn arg_root_dir() -> Arg<'static, 'static> {
	Arg::with_name("root_dir")
		.short("d")
		.takes_value(true)
		.help("The root dir where the driving kdd.yaml reside")
}

// endregion: Common Args
