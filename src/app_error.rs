use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
	#[error(transparent)]
	IOError(#[from] std::io::Error),

	#[error(transparent)]
	ClapError(#[from] clap::Error),

	#[error(transparent)]
	KddError(#[from] crate::kdd::error::KddError),
}
