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

use crate::error::ApiError;
use actix_web::http::header::HttpDate;
use actix_web::http::Method;
use rsa::{Hash, PaddingScheme, RsaPrivateKey};
use sha2::{Digest, Sha256};
use std::time::SystemTime;

// Returns the value of `Digest` header.
#[inline]
pub fn digest(data: impl AsRef<[u8]>) -> String {
	format!("SHA-256={}", base64::encode(Sha256::digest(data)))
}

// Returns the value of `Signature` header.
pub fn sign(
	key_id: &str,
	request_method: Method,
	request_path: &str,
	host: &str,
	date: SystemTime,
	body_digest: &str,
	private_key: &RsaPrivateKey,
) -> Result<String, ApiError> {
	let str_for_signing = format!(
		"(request-target): {} {}\nhost: {}\ndate: {}\ndigest: {}",
		request_method.to_string().to_lowercase(),
		request_path,
		host,
		HttpDate::from(date),
		body_digest
	);

	let digest = Sha256::digest(str_for_signing.as_bytes());
	let padding_scheme = PaddingScheme::new_pkcs1v15_sign(Some(Hash::SHA2_256));

	let signed_str = base64::encode(private_key.sign(padding_scheme, &digest)?);

	return Ok(format!("keyId=\"{}\",algorithm=\"rsa-sha256\",headers=\"(request-target) host date digest\",signature=\"{}\"", key_id, signed_str));
}
