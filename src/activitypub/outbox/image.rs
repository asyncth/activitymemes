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

use crate::activitypub::object_handlers;
use crate::activitypub::object_handlers::utils::{self, ToCcUuids};
use crate::error::ApiError;
use crate::state::AppState;
use crate::url;
use activitystreams::object::Image;
use activitystreams::primitives::XsdAnyUri;
use actix_web::http::header;
use actix_web::web;
use actix_web::HttpResponse;
use chrono::Utc;
use tracing::instrument;
use uuid::Uuid;

#[instrument]
pub async fn post_image(
	state: web::Data<AppState>,
	body: Image,
	user_id: Uuid,
	username: &str,
) -> Result<HttpResponse, ApiError> {
	let activity_id = Uuid::new_v4();
	let activity_url = url::activitypub_activity(activity_id);
	let actor_url = XsdAnyUri::try_from(url::activitypub_actor(username))?;

	let name = object_handlers::get_name(&body).ok_or(ApiError::OtherBadRequest)?;
	let summary = object_handlers::get_summary(&body);
	let image_url = object_handlers::get_url(&body).ok_or(ApiError::OtherBadRequest)?;

	let to = object_handlers::get_to(&body);
	let cc = object_handlers::get_cc(&body);

	let to = if let Some(to) = to {
		Some(utils::limit_to_and_cc(to.into_iter())?)
	} else {
		None
	};

	let cc = if let Some(cc) = cc {
		Some(utils::limit_to_and_cc(cc.into_iter())?)
	} else {
		None
	};

	let published_at = Utc::now();
	let new_image = object_handlers::new_image(
		activity_id,
		actor_url.clone(),
		name,
		summary,
		image_url.clone(),
		published_at,
		to.clone(),
		cc.clone(),
	)?;

	let activity = object_handlers::new_create(
		activity_id,
		actor_url,
		published_at,
		new_image.try_into()?,
		to.clone(),
		cc.clone(),
	)?;

	let to = if let Some(to) = to {
		Some(utils::actor_urls_to_uuids(state.clone(), to.iter()).await?)
	} else {
		None
	};

	let cc = if let Some(cc) = cc {
		Some(utils::actor_urls_to_uuids(state.clone(), cc.iter()).await?)
	} else {
		None
	};

	let (to, cc, is_public) = if to.is_some() && cc.is_some() {
		let to = to.unwrap();
		let cc = cc.unwrap();

		let to_has_public_uri = to.has_public_uri;
		let cc_has_public_uri = cc.has_public_uri;

		(to, cc, to_has_public_uri && cc_has_public_uri)
	} else if to.is_some() && cc.is_none() {
		let to = to.unwrap();
		let has_public_uri = to.has_public_uri;

		(to, ToCcUuids::default(), has_public_uri)
	} else if to.is_none() && cc.is_some() {
		let cc = cc.unwrap();
		let has_public_uri = cc.has_public_uri;

		(ToCcUuids::default(), cc, has_public_uri)
	} else {
		unreachable!();
	};

	let serialized_activity = serde_json::to_value(activity)?;
	sqlx::query("INSERT INTO activities (id, user_id, this_instance, published_at, activity, is_public, to_mentions, cc_mentions, to_followers_of, cc_followers_of) VALUES ($1, $2, TRUE, $3, $4, $5, $6, $7, $8, $9)")
		.bind(activity_id)
		.bind(user_id)
		.bind(published_at)
		.bind(serialized_activity)
		.bind(is_public)
		.bind(to.mentions)
		.bind(cc.mentions)
		.bind(to.followers_of)
		.bind(cc.followers_of)
		.execute(&state.db)
		.await?;

	Ok(HttpResponse::Created()
		.insert_header((header::LOCATION, activity_url))
		.finish())
}
