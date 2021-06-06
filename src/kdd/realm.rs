////////////////////////////////////
// kdd::realm - All realm related actions
////

use super::{error::KddError, Kdd, Realm};

impl<'a> Kdd<'a> {
	pub fn realm_for_ctx(&self, ctx: &str) -> Option<&Realm> {
		self.realms().into_iter().find(|v| v.context.as_deref().map(|vc| vc == ctx).unwrap_or(false))
	}

	pub fn current_realm(&self) -> Result<Option<&Realm>, KddError> {
		let ctx = self.k_current_context()?;
		// TODO: Set the project if realm found
		Ok(self.realm_for_ctx(&ctx))
	}

	pub fn realms(&self) -> Vec<&Realm> {
		self.realms.values().collect()
	}

	pub fn realm_set(&self, name: &str) -> Result<(), KddError> {
		match self.realms.get(name) {
			None => Err(KddError::RealmNotFound(name.to_string())),
			Some(realm) => match &realm.context {
				Some(ctx) => self.k_set_context(&ctx),
				None => Err(KddError::RealmHasNoContext(name.to_string())),
			},
		}
	}

	pub fn print_realms(&self) -> Result<(), KddError> {
		let current_realm = self.current_realm()?;
		let current_ctx = current_realm.map(|r| r.context.as_deref()).flatten();
		let realms = self.realms();
		tr_print(false, "REALM", "TYPE", "PROFILE/PROJECT", "CONTEXT");

		for realm in realms {
			let pr = realm.profile.as_deref().or(realm.project.as_deref()).unwrap_or("-");
			let ctx = realm.context.as_deref();
			let typ = realm.provider.to_string();
			let is_current = ctx.is_some() && current_ctx == ctx;
			let ctx = ctx.unwrap_or("-");
			tr_print(is_current, &realm.name, &typ, pr, ctx);
		}

		Ok(())
	}
}

// region:    Utils
fn tr_print(sel: bool, realm: &str, typ: &str, prj: &str, ctx: &str) {
	let sel = if sel { "*" } else { " " };
	println!("{}  {: <12}{: <14}{: <20}{}", sel, realm, typ, prj, ctx);
}
// endregion: Utils
