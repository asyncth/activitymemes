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
use actix_web::rt::time::Instant;
use jsonwebtoken::{DecodingKey, EncodingKey};
use rsa::RsaPrivateKey;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use std::collections::VecDeque;
use std::error::Error;
use std::fs;
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;
use tracing::instrument;
use url::Url;

pub struct AppState {
	pub scheme: String,
	pub domain: String,
	pub token_encoding_key: EncodingKey,
	pub token_decoding_key: DecodingKey,
	pub db: Pool<Postgres>,
	pub delivery_retry_queue: Mutex<VecDeque<FailedDelivery>>,
	pub delivery_retry_notify: Notify,
}

impl AppState {
	#[instrument]
	pub async fn new(config: Config) -> Result<Self, Box<dyn Error>> {
		let db = PgPoolOptions::new()
			.max_connections(config.num_of_db_pool_connections)
			.connect(&config.db_connection_uri)
			.await?;

		let token_encoding_key =
			EncodingKey::from_rsa_pem(&fs::read(config.token_rsa_private_key_pem_filepath)?)?;
		let token_decoding_key =
			DecodingKey::from_rsa_pem(&fs::read(config.token_rsa_public_key_pem_filepath)?)?;

		let delivery_retry_queue = Mutex::new(VecDeque::new());
		let delivery_retry_notify = Notify::new();

		Ok(Self {
			scheme: config.scheme,
			domain: config.domain,
			token_encoding_key,
			token_decoding_key,
			db,
			delivery_retry_queue,
			delivery_retry_notify,
		})
	}
}

#[derive(Clone, Debug)]
pub struct FailedDelivery {
	pub activity: Arc<serde_json::Value>,
	pub recipients: Vec<Url>,
	pub actor_id: Arc<String>,
	pub private_key: Arc<RsaPrivateKey>,
	pub digest: Arc<String>,
	pub last_time_tried: Instant,
	pub tried: u32,
}
