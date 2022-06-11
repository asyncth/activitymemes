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

use crate::account::{EMAIL_REGEX, RNG};
use crate::error::ApiError;
use crate::state::AppState;
use actix_web::{post, web, HttpResponse};
use once_cell::sync::Lazy;
use pbkdf2::{
	password_hash::{PasswordHasher, SaltString},
	Pbkdf2,
};
use regex::Regex;
use serde::Deserialize;
use sqlx::Row;
use tracing::instrument;
use uuid::Uuid;

static USERNAME_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap());

#[derive(Clone, Debug, Deserialize)]
pub struct PostSignUpBody {
	username: String,
	email: String,
	password: String,
}

#[post("/sign-up")]
#[instrument(skip(state))]
pub async fn post_sign_up(
	state: web::Data<AppState>,
	body: web::Json<PostSignUpBody>,
) -> Result<HttpResponse, ApiError> {
	if !USERNAME_REGEX.is_match(&body.username) {
		return Err(ApiError::InvalidUsername);
	}

	if !EMAIL_REGEX.is_match(&body.email) {
		return Err(ApiError::InvalidEmail);
	}

	let user_exists: bool = sqlx::query(
		"SELECT EXISTS(SELECT 1 FROM users WHERE username = $1 AND this_instance = TRUE)",
	)
	.bind(&body.username)
	.fetch_one(&state.db)
	.await?
	.get(0);
	if user_exists {
		return Err(ApiError::UsernameAlreadyTaken);
	}

	let email_already_used: bool =
		sqlx::query("SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)")
			.bind(&body.email)
			.fetch_one(&state.db)
			.await?
			.get(0);
	if email_already_used {
		return Err(ApiError::EmailAlreadyTaken);
	}

	if body.password.is_empty() {
		return Err(ApiError::PasswordMustNotBeEmpty);
	}

	let password_salt = RNG.with(|cell| SaltString::generate(&mut *cell.borrow_mut()));
	let hashed_password = Pbkdf2
		.hash_password(body.password.as_bytes(), &password_salt)
		.map(|hash| hash.to_string())?;

	let uuid = Uuid::new_v4();
	sqlx::query(
        "INSERT INTO users (id, username, this_instance, instance_url, email, password, name) VALUES ($1, $2, TRUE, NULL, $3, $4, $5)",
    )
    .bind(uuid)
    .bind(&body.username)
    .bind(&body.email)
    .bind(hashed_password)
    .bind(&body.username)
    .execute(&state.db)
    .await?;

	Ok(HttpResponse::Ok().finish())
}
