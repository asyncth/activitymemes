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

use crate::config::Config;
use jsonwebtoken::{DecodingKey as JwtDecodingKey, EncodingKey as JwtEncodingKey};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use std::error::Error;
use std::fmt::Debug;
use std::fs;
use tracing::instrument;

#[derive(Clone, Debug)]
pub struct AppState {
	pub scheme: String,
	pub domain: String,
	pub token_encoding_key: EncodingKey,
	pub token_decoding_key: DecodingKey,
	pub db: Pool<Postgres>,
}

impl AppState {
	#[instrument]
	pub async fn new(config: Config) -> Result<Self, Box<dyn Error>> {
		let db = PgPoolOptions::new()
			.max_connections(config.num_of_db_pool_connections)
			.connect(&config.db_connection_uri)
			.await?;

		let token_encoding_key = EncodingKey(JwtEncodingKey::from_rsa_pem(&fs::read(
			config.token_rsa_private_key_pem_filepath,
		)?)?);
		let token_decoding_key = DecodingKey(JwtDecodingKey::from_rsa_pem(&fs::read(
			config.token_rsa_public_key_pem_filepath,
		)?)?);

		Ok(Self {
			scheme: config.scheme,
			domain: config.domain,
			token_encoding_key,
			token_decoding_key,
			db,
		})
	}
}

#[derive(Clone)]
pub struct EncodingKey(JwtEncodingKey);

impl EncodingKey {
	pub fn inner(&self) -> &JwtEncodingKey {
		&self.0
	}
}

impl Debug for EncodingKey {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("EncodingKey").finish()
	}
}

#[derive(Clone)]
pub struct DecodingKey(JwtDecodingKey);

impl DecodingKey {
	pub fn inner(&self) -> &JwtDecodingKey {
		&self.0
	}
}

impl Debug for DecodingKey {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("DecodingKey").finish()
	}
}
