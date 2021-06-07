#![allow(unused)] // silence unused warnings while exploring (to comment out)

mod app_error;
mod cmd;
mod kdd;
mod utils;
mod yutils;

use crate::cmd::cmd_run;

fn main() {
	match cmd_run() {
		Ok(_) => println!("✔ All good and well"),
		Err(e) => {
			println!("Error:\n  {}", e)
		}
	};
}
