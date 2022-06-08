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
use crate::state::AppState;
use crate::url;
use activitystreams::primitives::XsdAnyUri;
use actix_web::web;
use futures::future;
use sqlx::Row;
use std::str::FromStr;
use tracing::instrument;
use uuid::Uuid;

/// Maximum number of `to` and `cc` mentions that aren't public addressing.
pub const MAX_NON_PUBLIC_CC_AND_TO: usize = 5;

#[derive(Clone, Debug, Default)]
pub struct ToCcUuids {
	pub mentions: Vec<Uuid>,
	pub followers_of: Vec<Uuid>,
	pub has_public_uri: bool,
}

#[derive(Clone, Debug)]
pub enum ToCcUuid {
	DirectMention(Uuid),
	MentionOfFollowersOf(Uuid),
}

#[instrument(skip(state, urls))]
pub async fn actor_urls_to_uuids<'a, I>(
	state: web::Data<AppState>,
	urls: I,
) -> Result<ToCcUuids, ApiError>
where
	I: IntoIterator<Item = &'a XsdAnyUri>,
{
	let urls = urls.into_iter();

	let mut uuids = ToCcUuids::default();
	let mut futures = Vec::with_capacity(urls.size_hint().0);

	for url in urls {
		let url = url.as_str();

		if url == "https://www.w3.org/ns/activitystreams#Public" {
			uuids.has_public_uri = true;
			continue;
		}

		futures.push(actor_url_to_uuid(state.clone(), url));
	}

	let to_cc_uuids = future::join_all(futures).await;
	for uuid in to_cc_uuids {
		let uuid = uuid?;
		match uuid {
			ToCcUuid::DirectMention(id) => uuids.mentions.push(id),
			ToCcUuid::MentionOfFollowersOf(id) => uuids.followers_of.push(id),
		}
	}

	Ok(uuids)
}

#[instrument(skip(state))]
async fn actor_url_to_uuid(state: web::Data<AppState>, url: &str) -> Result<ToCcUuid, ApiError> {
	if url == "https://www.w3.org/ns/activitystreams#Public" {
		return Err(ApiError::InternalServerError);
	}

	if let Some(captures) = url::user_url_regex().captures(url) {
		let username = captures.get(1).unwrap().as_str();

		let user_id: Option<Uuid> =
			sqlx::query("SELECT id FROM users WHERE username = $1 AND this_instance = TRUE")
				.bind(username)
				.fetch_optional(&state.db)
				.await?
				.map(|row| row.get(0));
		if user_id.is_none() {
			return Err(ApiError::UserDoesNotExist);
		}
		let user_id = user_id.unwrap();

		return Ok(ToCcUuid::DirectMention(user_id));
	}

	if let Some(captures) = url::user_followers_url_regex().captures(url) {
		let username = captures.get(1).unwrap().as_str().to_string();
		let user_id: Option<Uuid> =
			sqlx::query("SELECT id FROM users WHERE username = $1 AND this_instance = TRUE")
				.bind(&username)
				.fetch_optional(&state.db)
				.await?
				.map(|row| row.get(0));
		if user_id.is_none() {
			return Err(ApiError::UserDoesNotExist);
		}
		let user_id = user_id.unwrap();

		return Ok(ToCcUuid::MentionOfFollowersOf(user_id));
	}

	todo!("discovering foreign users");
}

pub fn limit_to_and_cc<'a, I>(iter: I) -> Result<Vec<XsdAnyUri>, ApiError>
where
	I: IntoIterator<Item = &'a XsdAnyUri>,
{
	let v: Vec<XsdAnyUri> = iter.into_iter().cloned().collect();
	let public_addressing = XsdAnyUri::from_str("https://www.w3.org/ns/activitystreams#Public")?;

	let contains_public_addressing_before_take = v.contains(&public_addressing);
	let mut v: Vec<XsdAnyUri> = v.into_iter().take(MAX_NON_PUBLIC_CC_AND_TO).collect();
	let contains_public_addressing_after_take = v.contains(&public_addressing);

	if contains_public_addressing_before_take && !contains_public_addressing_after_take {
		v.push(public_addressing);
	}

	Ok(v)
}
