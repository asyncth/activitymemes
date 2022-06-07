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

use crate::activitypub::collections::followers::{Data, Followers};
use crate::activitypub::collections::{Collection, Item};
use crate::activitypub::outbox::object::UnsanitizedObject;
use crate::error::ApiError;
use crate::state::AppState;
use crate::url;
use activitystreams::object::Image;
use actix_web::http::header;
use actix_web::web;
use actix_web::HttpResponse;
use async_recursion::async_recursion;
use futures::future;
use futures::StreamExt;
use sqlx::Row;
use tracing::instrument;
use uuid::Uuid;

#[instrument]
pub async fn post_image(
	state: web::Data<AppState>,
	body: Image,
	user_id: Uuid,
	username: &str,
) -> Result<HttpResponse, ApiError> {
	let id = Uuid::new_v4();
	let activity_url = url::activitypub_activity(id);
	let object_url = url::activitypub_object(id);
	let actor_url = url::activitypub_actor(username);

	let image = UnsanitizedObject::new(body).sanitize(&object_url, Some(&actor_url))?;
	let activity = image.activity(&activity_url, &actor_url)?;

	let published_at = activity
		.object_props
		.get_published()
		.unwrap()
		.as_datetime()
		.naive_utc();

	let to: Vec<Uuid> = future::join_all(
		activity
			.object_props
			.get_many_to_xsd_any_uris()
			.unwrap()
			.map(|url| actor_url_to_uuids(state.clone(), url.as_str(), false)),
	)
	.await
	.into_iter()
	.collect::<Result<Vec<Vec<Uuid>>, ApiError>>()?
	.into_iter()
	.flatten()
	.collect();

	let cc: Vec<Uuid> = future::join_all(
		activity
			.object_props
			.get_many_cc_xsd_any_uris()
			.unwrap()
			.map(|url| actor_url_to_uuids(state.clone(), url.as_str(), false)),
	)
	.await
	.into_iter()
	.collect::<Result<Vec<Vec<Uuid>>, ApiError>>()?
	.into_iter()
	.flatten()
	.collect();

	let serialized_activity = serde_json::to_value(activity)?;
	sqlx::query("INSERT INTO activities (id, user_id, this_instance, published_at, activity, is_public, to_mentions, cc_mentions) VALUES ($1, $2, TRUE, $3, $4, TRUE, $5, $6)")
		.bind(id)
		.bind(user_id)
		.bind(published_at)
		.bind(serialized_activity)
		.bind(to)
		.bind(cc)
		.execute(&state.db)
		.await?;

	Ok(HttpResponse::Created()
		.insert_header((header::LOCATION, activity_url))
		.finish())
}

#[async_recursion(?Send)]
async fn actor_url_to_uuids(
	state: web::Data<AppState>,
	url: &str,
	called_by_self: bool,
) -> Result<Vec<Uuid>, ApiError> {
	if url == "https://www.w3.org/ns/activitystreams#Public" {
		return Ok(Vec::new());
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

		return Ok(vec![user_id]);
	}

	if let Some(captures) = url::user_followers_url_regex().captures(url) {
		if called_by_self {
			return Err(ApiError::RecursionLimitReached);
		}

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

		let collection = Collection::new(Followers::new(state.clone()));
		let data = Data { user_id, username };

		let mut stream = collection.stream(&data);
		let mut urls = Vec::new();

		while let Some(item) = stream.next().await {
			let item = item?;
			match item {
				Item::XsdString(inner) => urls.push(inner.data),
				Item::BaseBox(_) => unreachable!(),
			}
		}

		let mut all_uuids = Vec::new();
		for url in urls {
			let uuids = actor_url_to_uuids(state.clone(), &url, true).await?;
			for uuid in uuids {
				all_uuids.push(uuid);
			}
		}

		return Ok(all_uuids);
	}

	todo!("discovering foreign users");
}
