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

use crate::activitypub::object_handlers::utils::{RemoteOrLocalId, ToCcUuids};
use crate::activitypub::object_handlers::{self, utils};
use crate::error::ApiError;
use crate::state::AppState;
use crate::{routines, url};
use activitystreams::activity::Create;
use activitystreams::object::kind::ImageType;
use activitystreams::object::Image;
use activitystreams::primitives::XsdAnyUri;
use activitystreams::BaseBox;
use actix_web::http::header;
use actix_web::{web, HttpResponse};
use chrono::Utc;
use rsa::pkcs8::DecodePrivateKey;
use rsa::RsaPrivateKey;
use sqlx::Row;
use std::collections::HashSet;
use tracing::instrument;
use uuid::Uuid;

#[instrument(skip(state, username))]
pub async fn post_create(
	state: web::Data<AppState>,
	body: Create,
	user_id: Uuid,
	username: &str,
) -> Result<HttpResponse, ApiError> {
	let activity_id = Uuid::new_v4();
	let activity_url = url::activitypub_activity(activity_id);
	let actor_url = XsdAnyUri::try_from(url::activitypub_actor(username))?;

	let activity_to = object_handlers::get_to(&body);
	let activity_cc = object_handlers::get_cc(&body);
	let inner_object =
		object_handlers::get_object_base_box(&body).ok_or(ApiError::OtherBadRequest)?;

	let published_at = Utc::now();

	let (inner_object, to, cc) = if inner_object.is_kind(ImageType) {
		let image: Image = inner_object.clone().into_concrete().unwrap();
		let name = object_handlers::get_name(&image).ok_or(ApiError::OtherBadRequest)?;
		let summary = object_handlers::get_summary(&image);
		let url = object_handlers::get_url(&image).ok_or(ApiError::OtherBadRequest)?;
		let object_to = object_handlers::get_to(&image);
		let object_cc = object_handlers::get_cc(&image);

		let object_to = object_to.unwrap_or_default();
		let object_cc = object_cc.unwrap_or_default();
		let activity_to = activity_to.unwrap_or_default();
		let activity_cc = activity_cc.unwrap_or_default();

		let (to, cc) = utils::merge_and_limit_mentions(
			object_to.into_iter(),
			object_cc.into_iter(),
			activity_to.into_iter(),
			activity_cc.into_iter(),
		)?;
		let new_image = object_handlers::new_image(
			activity_id,
			actor_url.clone(),
			name,
			summary,
			url.clone(),
			published_at,
			Some(to.clone()),
			Some(cc.clone()),
		)?;

		(BaseBox::try_from(new_image)?, to, cc)
	} else {
		return Err(ApiError::OtherBadRequest);
	};

	let new_create = object_handlers::new_create(
		activity_id,
		actor_url,
		published_at,
		inner_object,
		Some(to.clone()),
		Some(cc.clone()),
	)?;

	let to = utils::actor_urls_to_uuids(state.clone(), to.iter()).await?;
	let cc = utils::actor_urls_to_uuids(state.clone(), cc.iter()).await?;

	let mut deliver_to = HashSet::new();
	for id in &to.mentions {
		if let RemoteOrLocalId::Remote(_, url) = id {
			deliver_to.insert(url.clone());
		}
	}

	for id in &cc.mentions {
		if let RemoteOrLocalId::Remote(_, url) = id {
			deliver_to.insert(url.clone());
		}
	}

	let to: ToCcUuids = to.into();
	let cc: ToCcUuids = cc.into();

	let is_public = to.has_public_uri || cc.has_public_uri;

	let serialized_activity = serde_json::to_value(new_create)?;
	sqlx::query("INSERT INTO activities (id, user_id, this_instance, published_at, activity, is_public, to_mentions, cc_mentions, to_followers_of, cc_followers_of) VALUES ($1, $2, TRUE, $3, $4, $5, $6, $7, $8, $9)")
		.bind(activity_id)
		.bind(user_id)
		.bind(published_at)
		.bind(&serialized_activity)
		.bind(is_public)
		.bind(to.mentions)
		.bind(cc.mentions)
		.bind(to.followers_of)
		.bind(cc.followers_of)
		.execute(&state.db)
		.await?;

	if !deliver_to.is_empty() {
		let private_key_pem: String = sqlx::query("SELECT private_key FROM users WHERE id = $1")
			.bind(user_id)
			.fetch_one(&state.db)
			.await?
			.get(0);

		let private_key = RsaPrivateKey::from_pkcs8_pem(&private_key_pem)?;

		actix_web::rt::spawn(routines::deliver_activity(
			state.clone(),
			serialized_activity,
			deliver_to,
			url::activitypub_actor(username),
			private_key,
		));
	}

	Ok(HttpResponse::Created()
		.insert_header((header::LOCATION, activity_url))
		.finish())
}
