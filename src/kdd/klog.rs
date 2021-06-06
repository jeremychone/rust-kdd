////////////////////////////////////
// kdd::klog - implementation of the kubectl log on multiple services
////

use super::{error::KddError, Kdd, Realm};
use crate::utils::exec_to_stdout;
use serde_json::Value;
use std::{
	collections::{HashMap, HashSet},
	process::Stdio,
	time::Duration,
};
use tokio::{
	io::{AsyncBufReadExt, BufReader},
	process::Command,
	sync::mpsc::{self, Sender},
	time::timeout,
};

const BUF_LOG_CAPACITY: usize = 50;
const BUF_MSTIME_TO_LOG: u64 = 500;

#[derive(Debug)]
struct Pod {
	service_name: String,
	name: String,
}

#[derive(Debug, Clone)]
struct LogMessage {
	service_name: String,
	pod_name: String,
	line: String,
}

impl<'a> Kdd<'a> {
	pub fn k_log(&self, _realm: &Realm, names: Option<&[&str]>) -> Result<(), KddError> {
		let pods = Self::k_get_json_items("pod")?;
		let mut pods = Self::list_pods(&pods);

		//// filter the names
		if let Some(names) = names {
			let names_set: HashSet<String> = names.iter().map(|v| format!("{}-{}", self.system, v)).collect();
			pods = pods.into_iter().filter(|pod| names_set.contains(&pod.service_name)).collect();
		}

		show_klogs_for_pods(pods)?;

		Ok(())
	}

	// region:    Get K8s entities

	fn list_pods(json_pods: &Vec<Value>) -> Vec<Pod> {
		let mut pods: Vec<Pod> = Vec::new();

		for json_pod in json_pods {
			match (json_pod.pointer("/metadata/name"), json_pod.pointer("/metadata/labels/run")) {
				(Some(Value::String(pod_name)), Some(Value::String(service_name))) => {
					pods.push(Pod {
						name: pod_name.to_owned(),
						service_name: service_name.to_owned(),
					});
				}
				_ => {
					// println!("->> UNKNOWN\n{}\n\n", to_string_pretty(pod).unwrap());
				}
			}
		}
		pods
	}

	fn k_get_json_items(entity_type: &str) -> Result<Vec<Value>, KddError> {
		let args = &["get", entity_type, "-o", "json"];
		let json = exec_to_stdout(None, "kubectl", args)?;
		let mut json = serde_json::from_str::<Value>(&json)?;

		match json["items"].take() {
			Value::Array(items) => Ok(items),
			_ => Err(KddError::KGetObjectsEmpty(entity_type.to_string())),
		}
	}

	// endregion: Get K8s entities
}

#[tokio::main(flavor = "current_thread")]
async fn show_klogs_for_pods(pods: Vec<Pod>) -> Result<(), KddError> {
	let (tx, mut rx) = mpsc::channel::<LogMessage>(32);

	for pod in pods.into_iter() {
		let tx = tx.clone();
		tokio::spawn(async move {
			if let Err(ex) = show_pod_klog(pod, tx).await {
				println!("ERROR - Cannot start the kubectl log -f cause: {}", ex);
			};
		});
	}

	let mut buf: Vec<LogMessage> = Vec::with_capacity(BUF_LOG_CAPACITY);

	loop {
		// we reald for the buf mstime
		while let Ok(Some(log_message)) = timeout(Duration::from_millis(BUF_MSTIME_TO_LOG), rx.recv()).await {
			buf.push(log_message);
		}
		// then we print by service_name
		if buf.len() > 0 {
			// split the logs by service name
			let mut map: HashMap<String, Vec<LogMessage>> = HashMap::new();
			for log_message in buf.into_iter() {
				map.entry(log_message.service_name.to_string()).or_insert_with(Vec::new).push(log_message)
			}

			// print the result by service
			for (name, logs) in map.into_iter() {
				println!("=== Log for {}", name);
				for log in logs {
					println!("{} - {}", log.pod_name, log.line);
				}
				println!();
			}
			// Create new vector
			buf = Vec::with_capacity(BUF_LOG_CAPACITY);
		}
	}
}

async fn show_pod_klog(pod: Pod, tx: Sender<LogMessage>) -> Result<(), KddError> {
	println!("> Listening to Service {} (pod: {})", pod.name, pod.service_name);
	let cmd = "kubectl";
	let args = &["logs", "-f", &pod.name];

	let mut proc = Command::new(&cmd);
	let proc = proc.args(args).stdout(Stdio::piped());

	let mut child = proc.spawn()?;

	let stdout = child.stdout.take().expect("child did not have a handle to stdout");
	let mut reader = BufReader::new(stdout).lines();

	while let Some(line) = reader.next_line().await? {
		if let Err(_) = tx
			.send(LogMessage {
				service_name: pod.service_name.to_string(),
				pod_name: pod.name.to_string(),
				line: line,
			})
			.await
		{
			return Err(KddError::KLogTxSendError(pod.name.to_string()));
		};
	}

	Ok(())
}
