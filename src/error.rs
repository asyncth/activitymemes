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

use activitystreams::primitives::XsdAnyUriError;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use serde_json::json;
use std::convert::Infallible;
use std::fmt;
use tracing::error;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ApiError {
	IncorrectResourceQuery,
	UserDoesNotExist,
	InternalServerError,
	InvalidUsername,
	InvalidEmail,
	UsernameAlreadyTaken,
	EmailAlreadyTaken,
	PasswordMustNotBeEmpty,
	IncorrectPassword,
	NotSignedIn,
	Forbidden,
	ResourceNotFound,
	OtherBadRequest,
}

impl fmt::Display for ApiError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match *self {
			Self::IncorrectResourceQuery => {
				write!(f, "Incorrect resource query parameter.")
			}
			Self::UserDoesNotExist => write!(f, "User doesn't exist."),
			Self::InternalServerError => write!(f, "Internal server error."),
			Self::InvalidUsername => write!(f, "Invalid username."),
			Self::InvalidEmail => write!(f, "Invalid email."),
			Self::UsernameAlreadyTaken => write!(f, "Username is already taken."),
			Self::EmailAlreadyTaken => write!(f, "Email is already taken."),
			Self::PasswordMustNotBeEmpty => write!(f, "Password must not be empty."),
			Self::IncorrectPassword => write!(f, "Incorrect password."),
			Self::NotSignedIn => write!(f, "Not signed in."),
			Self::Forbidden => write!(f, "Forbidden."),
			Self::ResourceNotFound => write!(f, "Resource not found."),
			Self::OtherBadRequest => write!(f, "Bad request."),
		}
	}
}

impl std::error::Error for ApiError {}

impl ResponseError for ApiError {
	fn status_code(&self) -> StatusCode {
		match *self {
			Self::UserDoesNotExist => StatusCode::NOT_FOUND,
			Self::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
			Self::Forbidden => StatusCode::FORBIDDEN,
			Self::ResourceNotFound => StatusCode::NOT_FOUND,
			_ => StatusCode::BAD_REQUEST,
		}
	}

	fn error_response(&self) -> HttpResponse {
		let body = json!({ "error": self.to_string() });
		HttpResponse::build(self.status_code())
			.content_type("application/json")
			.json(body)
	}
}

impl From<sqlx::Error> for ApiError {
	fn from(err: sqlx::Error) -> Self {
		error!(?err, "SQL error");
		Self::InternalServerError
	}
}

impl From<XsdAnyUriError> for ApiError {
	fn from(err: XsdAnyUriError) -> Self {
		error!(?err, "Failed to parse XsdAnyUri");
		Self::InternalServerError
	}
}

impl From<std::io::Error> for ApiError {
	fn from(err: std::io::Error) -> Self {
		error!(?err, "IO error");
		Self::InternalServerError
	}
}

impl From<Infallible> for ApiError {
	fn from(_: Infallible) -> Self {
		unreachable!();
	}
}
