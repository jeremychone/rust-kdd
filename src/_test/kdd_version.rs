use crate::test_utils::*;
use std::error::Error;

#[test]
fn version() -> Result<(), Box<dyn Error>> {
	let mut kdd = load_kdd()?;
	let mut out = Vec::new();

	// update the version to DROP-001
	for version in kdd.versions.iter_mut() {
		version.by = "${1}HEYEHE${2}".to_string();
	}
	kdd.version(&mut out)?;
	let out_str = std::str::from_utf8(&out).unwrap();
	println!("{}", out_str);

	// reload kdd
	let kdd = load_kdd()?;
	let mut out = Vec::new();
	// and do normal version
	kdd.version(&mut out)?;
	let out_str = std::str::from_utf8(&out).unwrap();
	println!("{}", out_str);

	Ok(())
}
