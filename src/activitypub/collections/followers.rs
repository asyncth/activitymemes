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

use super::{ItemXsdString, Items, Provider};
use crate::error::ApiError;
use crate::state::AppState;
use crate::url;
use actix_web::web;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use sqlx::{postgres::PgRow, Row};
use std::borrow::Cow;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Data {
	pub user_id: Uuid,
	pub username: String,
}

#[derive(Clone, Debug)]
pub struct Followers {
	state: web::Data<AppState>,
}

impl Followers {
	pub fn new(state: web::Data<AppState>) -> Self {
		Self { state }
	}

	fn query_to_item(&self, row: PgRow) -> ItemXsdString {
		let following_since: NaiveDateTime = row.get(0);
		let username: &str = row.get(1);
		let this_instance: bool = row.get(2);
		let instance_url: Option<String> = row.get(3);

		let url = if this_instance {
			url::activitypub_actor(username)
		} else {
			instance_url.expect("expected `instance_url` to be not null")
		};

		ItemXsdString {
			id: following_since.timestamp_millis(),
			data: url,
		}
	}
}

#[async_trait(?Send)]
impl Provider for Followers {
	type Error = ApiError;
	type Data = Data;

	fn activitypub_id(&self, data: &Self::Data) -> Cow<'_, str> {
		Cow::Owned(format!(
			"{}/followers",
			url::activitypub_actor(&data.username)
		))
	}

	async fn total_items(&self, data: &Self::Data) -> Result<u64, Self::Error> {
		let total_items: i64 = sqlx::query(
			"SELECT COUNT(1) FROM follows WHERE object_user_id = $1 AND pending = FALSE",
		)
		.bind(data.user_id)
		.fetch_one(&self.state.db)
		.await?
		.get(0);

		let total_items = u64::try_from(total_items).expect("expected count to be zero or more");
		Ok(total_items)
	}

	async fn fetch_first_page(&self, data: &Self::Data) -> Result<Items, Self::Error> {
		let items: Vec<ItemXsdString> = sqlx::query("SELECT follows.following_since, users.username, users.this_instance, users.instance_url FROM follows, users WHERE follows.object_user_id = $1 AND follows.subject_user_id = users.id AND pending = FALSE ORDER BY follows.following_since DESC LIMIT 20")
			.bind(data.user_id)
			.map(|row| self.query_to_item(row))
			.fetch_all(&self.state.db)
			.await?;

		Ok(Items::XsdString(items))
	}

	async fn fetch_max_id(&self, max_id: i64, data: &Self::Data) -> Result<Items, Self::Error> {
		let items: Vec<ItemXsdString> = sqlx::query("SELECT follows.following_since, users.username, users.this_instance, users.instance_url FROM follows, users WHERE follows.object_user_id = $1 AND follows.subject_user_id = users.id AND pending = FALSE AND follows.following_since < $2 ORDER BY follows.following_since DESC LIMIT 20")
			.bind(data.user_id)
			.bind(max_id)
			.map(|row| self.query_to_item(row))
			.fetch_all(&self.state.db)
			.await?;

		Ok(Items::XsdString(items))
	}

	async fn fetch_min_id(&self, min_id: i64, data: &Self::Data) -> Result<Items, Self::Error> {
		let items: Vec<ItemXsdString> = sqlx::query("SELECT * FROM (SELECT follows.following_since, users.username, users.this_instance, users.instance_url FROM follows, users WHERE follows.object_user_id = $1 AND follows.subject_user_id = users.id AND pending = FALSE AND follows.following_since > $2 ORDER BY follows.following_since ASC LIMIT 20) AS tmp ORDER BY following_since DESC")
			.bind(data.user_id)
			.bind(min_id)
			.map(|row| self.query_to_item(row))
			.fetch_all(&self.state.db)
			.await?;

		Ok(Items::XsdString(items))
	}
}
