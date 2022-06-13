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
use actix_web::error::PayloadError;
use actix_web::http::StatusCode;
use actix_web::rt::task::JoinError;
use actix_web::{HttpResponse, ResponseError};
use awc::error::SendRequestError;
use jsonwebtoken::errors::Error as JwtError;
use pbkdf2::password_hash::Error as Pbkdf2Error;
use rsa::errors::Error as RsaError;
use rsa::pkcs8::spki::Error as RsaPkcs8SpkiError;
use rsa::pkcs8::Error as RsaPkcs8Error;
use serde_json::{json, Error as SerdeJsonError};
use std::convert::Infallible;
use std::fmt;
use std::num::TryFromIntError;
use tracing::error;
use url::ParseError;

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
	BadUrl,
	UnexpectedResponseFromFederatedServer,
	FailedDeliveryDueToNetworkError,
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
			Self::BadUrl => write!(f, "Bad URL."),
			Self::UnexpectedResponseFromFederatedServer => {
				write!(f, "Unexpected response from a federated server.")
			}
			Self::FailedDeliveryDueToNetworkError => {
				write!(f, "Failed delivery due to network error.")
			}
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

impl From<Pbkdf2Error> for ApiError {
	fn from(err: Pbkdf2Error) -> Self {
		error!(?err, "Password hashing error");
		Self::InternalServerError
	}
}

impl From<JwtError> for ApiError {
	fn from(err: JwtError) -> Self {
		error!(?err, "JWT error");
		Self::InternalServerError
	}
}

impl From<SerdeJsonError> for ApiError {
	fn from(err: SerdeJsonError) -> Self {
		error!(?err, "JSON error");
		Self::InternalServerError
	}
}

impl From<SendRequestError> for ApiError {
	fn from(err: SendRequestError) -> Self {
		error!(?err, "Failed to send request");
		Self::InternalServerError
	}
}

impl From<PayloadError> for ApiError {
	fn from(err: PayloadError) -> Self {
		error!(?err, "Failed to get the body of a HTTP response");
		Self::InternalServerError
	}
}

impl From<JoinError> for ApiError {
	fn from(err: JoinError) -> Self {
		error!(?err, "Failed to join tasks");
		Self::InternalServerError
	}
}

impl From<RsaError> for ApiError {
	fn from(err: RsaError) -> Self {
		error!(?err, "RSA error");
		Self::InternalServerError
	}
}

impl From<RsaPkcs8Error> for ApiError {
	fn from(err: RsaPkcs8Error) -> Self {
		error!(?err, "RSA PKCS#8 error");
		Self::InternalServerError
	}
}

impl From<RsaPkcs8SpkiError> for ApiError {
	fn from(err: RsaPkcs8SpkiError) -> Self {
		error!(?err, "RSA PKCS#8 SPKI error");
		Self::InternalServerError
	}
}

impl From<ParseError> for ApiError {
	fn from(_: ParseError) -> Self {
		Self::BadUrl
	}
}

impl From<TryFromIntError> for ApiError {
	fn from(err: TryFromIntError) -> Self {
		error!(?err, "Int conversion error");
		Self::InternalServerError
	}
}

impl From<Infallible> for ApiError {
	fn from(_: Infallible) -> Self {
		unreachable!();
	}
}
