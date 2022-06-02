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

use serde::Deserialize;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
	pub scheme: String,
	pub domain: String,
	pub db_connection_uri: String,
	pub num_of_db_pool_connections: u32,
	pub token_rsa_public_key_pem_filepath: String,
	pub token_rsa_private_key_pem_filepath: String,
}

impl Config {
	pub fn with_file(path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
		let reader = BufReader::new(File::open(path)?);
		let val: Self = serde_json::from_reader(reader)?;

		Ok(val)
	}
}
