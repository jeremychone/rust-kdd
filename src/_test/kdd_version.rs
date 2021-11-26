use regex::Regex;

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

#[test]
fn version_href_rgx() -> Result<(), Box<dyn Error>> {
	let content = r#"
	<link href="/css/all-bundle.css?v=DROP-003-SNAPSHOT" rel="stylesheet">
	"#;

	let val = r#"<.*(?:href|src).*?v=(.*?)(?:"|\&)"#;
	let val_rgx = Regex::new(&val)?;
	let replace = r#"(<.*(?:href|src).*?v=).+?("|\&.*)"#;
	let replace_rgx = Regex::new(&replace)?;

	let by = r#"${1}DROP-999${2}"#;

	// DO - extract orginal value
	let org_val = val_rgx
		.captures(&content)
		.map(|caps| caps.get(caps.len() - 1).map(|m| m.as_str()))
		.flatten();

	// CHECK orignial value
	assert_eq!("DROP-003-SNAPSHOT", org_val.unwrap_or(""));

	// DO - replace content
	let content = replace_rgx.replace_all(&content, by);
	// extract new version
	assert!(
		content.contains(r#"<link href="/css/all-bundle.css?v=DROP-999" rel="stylesheet">"#),
		"should contain '...?v=DROP-999'"
	);

	// DO - extract new value
	let new_val = val_rgx
		.captures(&content)
		.map(|caps| caps.get(caps.len() - 1).map(|m| m.as_str()))
		.flatten();
	// CHECK original value
	assert_eq!("DROP-999", new_val.unwrap_or(""));

	Ok(())
}
