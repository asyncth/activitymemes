// ActivityMemes - open-source federated meme-sharing platform.
// Copyright (C) 2022 asyncth
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, version 3 of the License.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use crate::state::AppState;
use actix_web::HttpRequest;
use jsonwebtoken::{Algorithm, Validation};
use once_cell::sync::Lazy;
use rand::rngs::ThreadRng;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

pub mod sign_in;
pub mod sign_out;
pub mod sign_up;

static EMAIL_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\S+@\S+\.\S+$").unwrap());

thread_local! {
	pub static RNG: RefCell<ThreadRng> = RefCell::new(ThreadRng::default());
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Claims {
	/// Expiration time. Unix timestamp.
	pub exp: usize,
	/// Issued at. Unix timestamp.
	pub iat: usize,
	/// Issuer. Currently always "ActivityMemes Account Sign-In".
	pub iss: String,
	/// Username.
	pub sub: String,
}

/// Returns `Some(username)` if signed in.
/// Otherwise, returns `None`.
pub fn ensure_signed_in(state: &AppState, req: &HttpRequest) -> Option<String> {
	if let Some(cookie) = req.cookie("session") {
		let token = cookie.value();
		let mut validation = Validation::new(Algorithm::RS512);
		validation.set_required_spec_claims(&["exp", "iss"]);
		validation.set_issuer(&["ActivityMemes Account Sign-In"]);

		let decoded =
			jsonwebtoken::decode::<Claims>(token, state.token_decoding_key.inner(), &validation)
				.ok()?;
		let username = decoded.claims.sub;

		Some(username)
	} else {
		None
	}
}
