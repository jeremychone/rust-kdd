////////////////////////////////////
// kdd::klog - implementation of the kubectl log on multiple services
////

use super::kevents::{monitor_kube_events, KubeEvent};
use super::PodsProvider;
use super::{error::KddError, Kdd, Pod, Realm};
use std::collections::{HashMap, HashSet};
use std::format as f;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc::{self, Sender};
use tokio::time::{sleep, timeout};

const BUF_LOG_CAPACITY: usize = 50;
const BUF_MSTIME_TO_LOG: u64 = 500;

#[derive(Debug)]
struct LogMessage {
	service_name: String,
	pod_name: String,
	line: String,
}

impl Kdd {
	/// Command executor for 'kdd klogs ...'
	pub fn k_log(&self, _realm: &Realm, names: Option<Vec<String>>) -> Result<(), KddError> {
		show_klogs_for_pods(self, names)?;
		Ok(())
	}
}

// TODO: Should try to reduce the required thread. main_thread/single thread seems to get stuck, but should not need much.
#[tokio::main]
async fn show_klogs_for_pods(kdd: &Kdd, service_names: Option<Vec<String>>) -> Result<(), KddError> {
	// TODO: (low) needs to define optimum channel buffer size
	let (log_tx, mut log_rx) = mpsc::channel::<LogMessage>(32);

	let (kube_events_tx, mut kube_events_rx) = mpsc::channel::<KubeEvent>(32);

	// --- Start the initial pod monitors
	let pods_provider = kdd.get_pods_provider();
	let pod_names = refresh_pod_monitors(&pods_provider, &service_names, None, log_tx.clone(), kube_events_tx.clone()).await?;

	// --- Listen to pod events and do refresh_pod_monitors as needed
	monitor_kube_events(kube_events_tx.clone()).await?;

	tokio::spawn(async move {
		let mut pod_names = pod_names;
		let kube_events_tx = kube_events_tx.clone();
		while let Some(kube_event) = kube_events_rx.recv().await {
			match kube_event {
				// The monitored stopped, so we can remove it from the list
				KubeEvent::PodLogFail(pod_name) => {
					// NOTE - If we do not put this print (or probably a sleep) he will not reconnect when the pod restart
					//        Very weird.
					println!("");
					pod_names.remove(&pod_name);
				}
				KubeEvent::Pod(pod_event) => {
					if pod_event.reason == "Started" {
						// When restart, it says started, but the kubectl logs is not ready yet
						sleep(Duration::from_secs(4)).await;

						if let Ok(pn) = refresh_pod_monitors(
							&pods_provider,
							&service_names,
							Some(&pod_names),
							log_tx.clone(),
							kube_events_tx.clone(),
						)
						.await
						{
							pod_names = pn
						}
					}
				}
				_ => (),
			}
		}
	});

	// --- Get all of the lines for all monitored pods, and categorize by service_name and print them
	let mut buf: Vec<LogMessage> = Vec::with_capacity(BUF_LOG_CAPACITY);

	loop {
		// -- Read all of the message send to the message reciever for a give ammount of time or size (log_rx)
		while let Ok(Some(log_message)) = timeout(Duration::from_millis(BUF_MSTIME_TO_LOG), log_rx.recv()).await {
			buf.push(log_message);
		}

		// -- Print the log messages by service name
		if buf.len() > 0 {
			// split the logs by service name
			let mut map: HashMap<String, Vec<LogMessage>> = HashMap::new();
			for log_message in buf.into_iter() {
				map
					.entry(log_message.service_name.to_string())
					.or_insert_with(Vec::new)
					.push(log_message)
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

async fn refresh_pod_monitors(
	pods_provider: &PodsProvider,
	service_names: &Option<Vec<String>>,
	except_pod_names: Option<&HashSet<String>>,
	log_tx: Sender<LogMessage>,
	kube_events_tx: Sender<KubeEvent>,
) -> Result<HashSet<String>, KddError> {
	// get the all the pod names
	let pods = pods_provider.get_pods_by_service_names(&service_names)?;
	let pod_names: HashSet<String> = pods.iter().map(|p| p.name.to_owned()).collect();

	// for all of
	for pod in pods.into_iter() {
		// if it is not part of the except_pod_names set
		if !except_pod_names.as_ref().map(|set| set.contains(&pod.name)).unwrap_or(false) {
			let log_tx = log_tx.clone();
			let kube_events_tx = kube_events_tx.clone();
			tokio::spawn(async move {
				let pod_name = pod.name.clone();

				let pod = Arc::new(pod);

				if let Err(ex) = monitor_pod(pod.clone(), log_tx.clone()).await {
					println!("KDD WARNING - fail kubectl log pod {}. Cause: {}", pod.name, ex);
				};

				// if here, means pod might have gone down
				println!(
					"KDD INFO - Pod '{}' not available for klogs anymore. Probably was removed by kubernetes.",
					pod_name
				);

				// If here, means,
				// NOTE - Right now, nobody is listening this event, as it is handled by the 'Listen to pod events' code block.
				kube_events_tx.send(KubeEvent::PodLogFail(pod_name)).await;
			});
		}
	}

	Ok(pod_names)
}

/// Will do a kubectl log for a given pod, and send LogMessage to tx
async fn monitor_pod(pod: Arc<Pod>, tx: Sender<LogMessage>) -> Result<(), KddError> {
	println!("> kubectl logs -f {}", pod.name);
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
