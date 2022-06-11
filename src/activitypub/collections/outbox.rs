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

use super::{ItemBaseBox, Items, Provider};
use crate::error::ApiError;
use crate::state::AppState;
use crate::url;
use activitystreams::BaseBox;
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

#[derive(Clone)]
pub struct Outbox {
	state: web::Data<AppState>,
}

impl Outbox {
	pub fn new(state: web::Data<AppState>) -> Self {
		Self { state }
	}

	fn query_to_item(&self, row: PgRow) -> Result<ItemBaseBox, ApiError> {
		let published_at: NaiveDateTime = row.get(0);
		let activity: Result<BaseBox, _> = serde_json::from_value(row.get(1));

		if let Ok(activity) = activity {
			Ok(ItemBaseBox {
				id: published_at.timestamp_millis(),
				data: activity,
			})
		} else {
			Err(ApiError::InternalServerError)
		}
	}
}

#[async_trait(?Send)]
impl Provider for Outbox {
	type Error = ApiError;
	type Data = Data;

	fn activitypub_id(&self, data: &Self::Data) -> Cow<'_, str> {
		Cow::Owned(format!("{}/outbox", url::activitypub_actor(&data.username)))
	}

	async fn total_items(&self, data: &Self::Data) -> Result<u64, Self::Error> {
		let total_items: i64 = sqlx::query("SELECT COUNT(1) FROM activities WHERE user_id = $1")
			.bind(data.user_id)
			.fetch_one(&self.state.db)
			.await?
			.get(0);

		let total_items = u64::try_from(total_items).expect("expected count to be zero or more");
		Ok(total_items)
	}

	async fn fetch_first_page(&self, data: &Self::Data) -> Result<Items, Self::Error> {
		let items: Result<Vec<ItemBaseBox>, ApiError> = sqlx::query("SELECT published_at, activity FROM activities WHERE user_id = $1 ORDER BY published_at DESC LIMIT 20")
			.bind(data.user_id)
			.map(|row| self.query_to_item(row))
			.fetch_all(&self.state.db)
			.await?
			.into_iter()
			.collect();

		Ok(Items::BaseBox(items?))
	}

	async fn fetch_max_id(&self, max_id: i64, data: &Self::Data) -> Result<Items, Self::Error> {
		let max_id =
			NaiveDateTime::from_timestamp(max_id / 1000, u32::try_from((max_id % 1000) * 1000000)?);
		let items: Result<Vec<ItemBaseBox>, ApiError> = sqlx::query("SELECT published_at, activity FROM activities WHERE user_id = $1 AND published_at < $2 ORDER BY published_at DESC LIMIT 20")
			.bind(data.user_id)
			.bind(max_id)
			.map(|row| self.query_to_item(row))
			.fetch_all(&self.state.db)
			.await?
			.into_iter()
			.collect();

		Ok(Items::BaseBox(items?))
	}

	async fn fetch_min_id(&self, min_id: i64, data: &Self::Data) -> Result<Items, Self::Error> {
		let min_id =
			NaiveDateTime::from_timestamp(min_id / 1000, u32::try_from((min_id % 1000) * 1000000)?);
		let items: Result<Vec<ItemBaseBox>, ApiError> = sqlx::query("SELECT * FROM (SELECT published_at, activity FROM activities WHERE user_id = $1 AND published_at > $2 ORDER BY published_at LIMIT 20) AS tmp ORDER BY published_at DESC")
			.bind(data.user_id)
			.bind(min_id)
			.map(|row| self.query_to_item(row))
			.fetch_all(&self.state.db)
			.await?
			.into_iter()
			.collect();

		Ok(Items::BaseBox(items?))
	}
}
