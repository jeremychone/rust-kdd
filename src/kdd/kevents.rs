use super::PodsProvider;
use super::{error::KddError, Kdd, Pod, Realm};
use crate::utils::jsons;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::format as f;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc::{self, Sender};
use tokio::time::timeout;

#[derive(Debug)]
pub enum KubeEvent {
	// --- kubectl get events variants
	Pod(KubeEventData),
	Other(KubeEventData),
	UnknownJson(Value),
	UnknownText(String),

	// --- Application KubeEvents not part of the kubectl
	PodLogFail(String), // pod_name
}

impl KubeEvent {
	fn from_str(data: &str) -> Self {
		match serde_json::from_str::<Value>(data) {
			Ok(value) => {
				let kind = jsons::as_string(&value, "/involvedObject/kind");
				let name = jsons::as_string(&value, "/involvedObject/name");
				let reason = jsons::as_string(&value, "/reason");
				let message = jsons::as_string(&value, "/message");
				let timestamp = jsons::as_string(&value, "/lastTimestamp");
				if let (Some(kind), Some(name), Some(reason), Some(message), Some(timestamp)) = (kind, name, reason, message, timestamp) {
					let data = KubeEventData {
						kind,
						name,
						reason,
						message,
						timestamp,
					};
					if data.kind == "Pod" {
						KubeEvent::Pod(data)
					} else {
						KubeEvent::Other(data)
					}
				} else {
					KubeEvent::UnknownJson(value)
				}
			}
			Err(_) => KubeEvent::UnknownText(data.to_string()),
		}
	}
}

#[derive(Debug)]
pub struct KubeEventData {
	pub kind: String,
	pub name: String,
	pub reason: String,
	pub message: String,
	pub timestamp: String,
}

pub async fn monitor_kube_events(events_tx: Sender<KubeEvent>) -> Result<(), KddError> {
	async fn inner(events_tx: Sender<KubeEvent>) -> Result<(), KddError> {
		let cmd = "kubectl";
		let args = &["get", "events", "--watch-only", "-o", "json"];
		let mut proc = Command::new(&cmd);
		let proc = proc.args(args).stdout(Stdio::piped());

		let mut child = proc.spawn()?;

		let stdout = child
			.stdout
			.take()
			.ok_or_else(|| KddError::FailExecProc(f!("no stdout to take, for {:?}", cmd)))?;
		let mut reader = BufReader::new(stdout).lines();

		let mut json_block: Option<String> = None;

		while let Some(line) = reader.next_line().await? {
			let mut block_ended = false;
			if line.starts_with("{") {
				let mut json = String::from(line);
				json.push('\n');
				json_block = Some(json);
			} else if let Some(json_block) = json_block.as_mut() {
				if line.trim().ends_with("}") {
					json_block.push_str(&line);
					block_ended = true
				} else {
					json_block.push_str(&line);
					json_block.push('\n');
				}
			} else {
				json_block = None;
			}

			if block_ended {
				if let Some(json_txt) = json_block {
					let event = KubeEvent::from_str(&json_txt);
					// println!("KDD POD EVENTS - JSON BLOCK ENDED. {:?}", event);
					json_block = None;
					events_tx.send(event).await;
				}
			}
		}

		Ok(())
	}

	tokio::spawn(async move {
		match inner(events_tx).await {
			Ok(_) => println!("KDD INFO - Done listening to pods"),
			Err(ex) => println!("KDD INFO - Error while listening to pods - {}", ex),
		}
		println!("> Stop listening to pods events");
	});

	Ok(())
}
