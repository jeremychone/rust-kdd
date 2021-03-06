use handlebars::TemplateError;
use thiserror::Error;

use crate::utils::UtilsError;

#[derive(Error, Debug)]
pub enum KddError {
	#[error("No kdd.yaml file found at {0}")]
	NoKddFileFound(String),

	#[error("kdd.yaml must have one and two document (for for vars and the other for the document itself)")]
	KddYamlInvalid,

	#[error("Invalid builder '{0}'. {1}")]
	InvalidBuilder(String, String),

	#[error("Invalid builder exec '{0}'. {1} ")]
	InvalidBuilderExec(String, String),

	#[error("No .yaml_dir property for realm '{0}'")]
	FailLoadNoK8sYamlDir(String),

	#[error("kdd.yaml failed to parse. Cause: {0}")]
	KdevFailToParseInvalid(String),

	#[error("kdd.yaml must have a system property")]
	NoSystem,

	#[error("Fail to set the realm {0}")]
	FailSetRealm(String),

	#[error("Missing 'context' property for realm '{0}'")]
	MissingRealmContext(String),

	#[error("Block {0} unknown. Build aborted")]
	BlockUnknown(String),

	#[error("Cannot dpush, no current realm")]
	DpushFailNoRealm,

	#[error("Cannot dpush, cause: {0}")]
	DpushFailed(String),

	#[error("Kubernetes objects not found for time '{0}'")]
	KGetObjectsEmpty(String),

	#[error("klog error while tx.send log message (pod name: {0})")]
	KLogTxSendError(String),

	#[error("Context '{0}' not supported")]
	ContextNotSupported(String),

	#[error("Fail to render k8s file '{0}' cause: {1}")]
	KtemplateFailRender(String, String),

	#[error("No exec.cmd found")]
	NoExecCmd,

	#[error("Fail exec proc, Cause: {0}")]
	FailExecProc(String),

	#[error("ERROR - Fail to execute. Cause: {0}")]
	FailDockerBuilder(String),

	#[error("Fail to execute, cause: {0}")]
	KubectlFail(String),

	#[error("Realm {0} not found")]
	RealmNotFound(String),

	#[error("Realm {0} has no kubernetes context (make sure this realm .context is set in the kdd.yaml)")]
	RealmHasNoContext(String),

	#[error("'aws ecr describe-repositories' failed cause: {0}")]
	AwsEcrDescribeRepositoriesFailed(String),

	#[error(transparent)]
	UtilsError(#[from] UtilsError),

	#[error(transparent)]
	IOError(#[from] std::io::Error),

	#[error(transparent)]
	YamlError(#[from] yaml_rust::ScanError),

	#[error(transparent)]
	HbsTemplateError(#[from] TemplateError),

	#[error(transparent)]
	JsonError(#[from] serde_json::Error),

	#[error("Cannot execute builder - cause: {0} ")]
	CannotExecute(String),
}
