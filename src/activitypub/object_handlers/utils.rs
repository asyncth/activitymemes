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
use crate::{routines, url as crate_url};
use activitystreams::primitives::XsdAnyUri;
use actix_web::web;
use futures::future;
use sqlx::Row;
use std::collections::HashSet;
use std::str::FromStr;
use tracing::instrument;
use url::Url;
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

	// If the URL points to a same-instance user.
	if let Some(captures) = crate_url::user_url_regex().captures(url) {
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

	// If the URL points to the followers collection of a same-instance user.
	if let Some(captures) = crate_url::user_followers_url_regex().captures(url) {
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

	// If the URL points to neither, see if it has our domain.
	let url = Url::parse(url)?;
	if let Some(domain) = url.domain() {
		if domain == state.domain {
			return Err(ApiError::OtherBadRequest);
		}
	}

	// If not, try to fetch it from the database if it's saved there already
	// or fetch it from the URL.
	let user_id = routines::fetch_remote_actor(&state, url).await?;
	Ok(ToCcUuid::DirectMention(user_id))
}

pub fn limit_to_and_cc<'a, I>(iter: I) -> Result<Vec<XsdAnyUri>, ApiError>
where
	I: IntoIterator<Item = &'a XsdAnyUri>,
{
	let v: Vec<&XsdAnyUri> = iter.into_iter().collect();
	let public_addressing = XsdAnyUri::from_str("https://www.w3.org/ns/activitystreams#Public")?;

	let contains_public_addressing_before_take = v.contains(&&public_addressing);
	let mut v: Vec<XsdAnyUri> = v
		.into_iter()
		.take(MAX_NON_PUBLIC_CC_AND_TO)
		.cloned()
		.collect();
	let contains_public_addressing_after_take = v.contains(&public_addressing);

	if contains_public_addressing_before_take && !contains_public_addressing_after_take {
		v.push(public_addressing);
	}

	Ok(v)
}

pub fn merge_and_limit_mentions<'a, T>(
	object_to: T,
	object_cc: T,
	activity_to: T,
	activity_cc: T,
) -> Result<(Vec<XsdAnyUri>, Vec<XsdAnyUri>), ApiError>
where
	T: IntoIterator<Item = &'a XsdAnyUri>,
{
	let mut to_deduplicated = HashSet::new();
	let mut cc_deduplicated = HashSet::new();

	for uri in object_to {
		to_deduplicated.insert(uri);
	}

	for uri in object_cc {
		cc_deduplicated.insert(uri);
	}

	for uri in activity_to {
		to_deduplicated.insert(uri);
	}

	for uri in activity_cc {
		cc_deduplicated.insert(uri);
	}

	let to = limit_to_and_cc(to_deduplicated.into_iter())?;
	let cc = limit_to_and_cc(cc_deduplicated.into_iter())?;

	Ok((to, cc))
}
