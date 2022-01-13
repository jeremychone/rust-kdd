use handlebars::{Context, Handlebars, RenderContext, Renderable};
use handlebars::{Output, Template};
use std::collections::HashMap;
use std::io::Error as IOError;
use std::string::FromUtf8Error;
use yaml_rust::Yaml;

// region:    Stmpl
#[allow(unused)]
#[derive(Debug)]
pub enum Stmpl {
	Plain(String),
	Tmpl(Template, String),
}

#[allow(unused)]
impl Stmpl {
	pub fn render(&self, hbs: &Handlebars, vars: &HashMap<String, String>) -> String {
		match self {
			Stmpl::Plain(txt) => txt.to_owned(),
			Stmpl::Tmpl(tmpl, org_txt) => {
				if let Ok(ctx) = Context::wraps(vars) {
					let mut rc = RenderContext::new(None);
					let mut out = StringOutput::new();
					match tmpl.render(hbs, &ctx, &mut rc, &mut out) {
						Ok(_) => out.into_string().unwrap(),
						Err(_) => org_txt.to_owned(),
					}
				} else {
					// fall back on the text
					org_txt.to_owned()
				}
			}
		}
	}
}
fn _as_stmpl(yaml: &Yaml, key: &str) -> Option<Stmpl> {
	let yaml = &yaml[key];
	yaml.as_str().map(|val| {
		let val = if val.contains("{{") {
			match Template::compile(val) {
				Ok(tmpl) => Stmpl::Tmpl(tmpl, val.to_string()),
				Err(_) => Stmpl::Plain(val.to_owned()),
			}
		} else {
			Stmpl::Plain(val.to_owned())
		};
		val
	})
}
// endregion: Stmpl

/// Returns Some(yaml) if the yaml is a hash with at least one key
pub fn as_yaml_map(yaml: Yaml) -> Option<Yaml> {
	if let Yaml::Hash(hash) = &yaml {
		if hash.len() > 0 {
			return Some(yaml);
		}
	}
	None
}

pub fn as_bool(yaml: &Yaml, key: &str) -> Option<bool> {
	yaml[key].as_bool()
}

#[inline(always)]
pub fn as_string(yaml: &Yaml, key: &str) -> Option<String> {
	yaml[key].as_str().map(|str| str.to_string())
}

/// serialiaze this yaml item (if string, bool, number) as string
pub fn to_string(yaml: &Yaml) -> Option<String> {
	if let Some(val) = yaml.as_str() {
		return Some(val.to_string());
	}

	if let Some(val) = yaml.as_f64() {
		return Some(val.to_string());
	}

	if let Some(val) = yaml.as_i64() {
		return Some(val.to_string());
	}
	if let Some(val) = yaml.as_bool() {
		return Some(val.to_string());
	}
	None
}

/// Returns a Some of vector of owned string
pub fn as_strings(yaml: &Yaml, key: &str) -> Option<Vec<String>> {
	let yaml = &yaml[key];
	// FIXME supports vec of strings
	if let Some(val) = yaml.as_str() {
		Some(vec![val.to_string()])
	} else if let Some(vals) = yaml.as_vec() {
		let strings = vals.into_iter().filter_map(|x| x.as_str().map(|x| x.to_owned())).collect();
		Some(strings)
	} else {
		None
	}
}

/// Remove the keys for a given Yaml (must take owner ship, might return self or another object)
// thanks to: https://github.com/chyh1990/yaml-rust/issues/123#issuecomment-827007230
pub fn remove_keys(yaml: Yaml, keys: &[&str]) -> Yaml {
	if let Yaml::Hash(mut hash) = yaml {
		for key in keys {
			let key = &Yaml::String((*key).into());
			if hash.contains_key(key) {
				hash.remove(key);
			}
		}

		Yaml::Hash(hash)
	} else {
		yaml
	}
}

//// Merge in place a extra yaml to a target
pub fn merge_yaml(target: &mut Yaml, extra: &Yaml, overwrite: bool) {
	if let (Yaml::Hash(target), Yaml::Hash(extra)) = (target, extra) {
		for key in extra.keys() {
			// if overwrite is fals, update target only if it does not contain the key
			if overwrite || !target.contains_key(key) {
				target.insert(key.clone(), extra.get(key).unwrap().clone());
			}
		}
	}
}

// region:    Handlebars Utils
// Note: Had to copy this struct/impl from rust-handlebars since it was not pub
//       Hopefully, will be made public: https://github.com/sunng87/handlebars-rust/issues/442
pub struct StringOutput {
	buf: Vec<u8>,
}

impl Output for StringOutput {
	fn write(&mut self, seg: &str) -> Result<(), IOError> {
		self.buf.extend_from_slice(seg.as_bytes());
		Ok(())
	}
}

#[allow(unused)] // FOR ot not used
impl StringOutput {
	pub fn new() -> StringOutput {
		StringOutput {
			buf: Vec::with_capacity(8 * 1024),
		}
	}

	pub fn into_string(self) -> Result<String, FromUtf8Error> {
		String::from_utf8(self.buf)
	}
}

// endregion: Handlebars Utils
