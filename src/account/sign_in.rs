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

use crate::account::{Claims, EMAIL_REGEX};
use crate::error::ApiError;
use crate::AppState;
use actix_web::cookie::time::Duration as CookieDuration;
use actix_web::cookie::{Cookie, SameSite};
use actix_web::{post, web, HttpResponse};
use chrono::{Duration as ChronoDuration, Utc};
use jsonwebtoken::{Algorithm, Header};
use pbkdf2::{
	password_hash::{PasswordHash, PasswordVerifier},
	Pbkdf2,
};
use serde::Deserialize;
use sqlx::Row;
use tracing::{error, instrument};

#[derive(Clone, Debug, Deserialize)]
pub struct SignInBody {
	email: String,
	password: String,
}

#[post("/sign-in")]
#[instrument]
pub async fn sign_in(
	state: web::Data<AppState>,
	body: web::Json<SignInBody>,
) -> Result<HttpResponse, ApiError> {
	if !EMAIL_REGEX.is_match(&body.email) {
		return Err(ApiError::InvalidEmail);
	}

	let user_exists: bool =
		sqlx::query("SELECT EXISTS(SELECT 1 FROM users WHERE email = $1 AND this_instance = TRUE)")
			.bind(&body.email)
			.fetch_one(&state.db)
			.await?
			.get(0);
	if !user_exists {
		return Err(ApiError::UserDoesNotExist);
	}

	let (username, hashed_password) = {
		let query = sqlx::query(
			"SELECT username, password FROM users WHERE email = $1 AND this_instance = TRUE",
		)
		.bind(&body.email)
		.fetch_one(&state.db)
		.await?;

		let username: String = query.get(0);
		let hashed_password: String = query.get(1);

		(username, hashed_password)
	};

	let parsed_hash = PasswordHash::new(&hashed_password).map_err(|e| {
		error!(?e, "Failed to parse hash");
		ApiError::InternalServerError
	})?;

	if Pbkdf2
		.verify_password(body.password.as_bytes(), &parsed_hash)
		.is_err()
	{
		return Err(ApiError::IncorrectPassword);
	}

	let now = Utc::now();
	let expiration_date = now.checked_add_signed(ChronoDuration::days(30)).unwrap();

	let claims = Claims {
		exp: expiration_date.timestamp() as usize,
		iat: now.timestamp() as usize,
		iss: String::from("ActivityMemes Account Sign-In"),
		sub: username,
	};

	let token = jsonwebtoken::encode(
		&Header::new(Algorithm::RS512),
		&claims,
		state.token_encoding_key.inner(),
	)
	.map_err(|e| {
		error!(?e, "Failed to encode claims");
		ApiError::InternalServerError
	})?;

	Ok(HttpResponse::Ok()
		.cookie(
			Cookie::build("session", token)
				.domain(&state.domain)
				.max_age(CookieDuration::seconds(86400 * 30))
				.same_site(SameSite::Lax)
				.secure(true)
				.http_only(true)
				.finish(),
		)
		.finish())
}
