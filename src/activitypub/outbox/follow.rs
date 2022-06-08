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
use activitystreams::activity::Follow;
use activitystreams::{
	activity::properties::FollowProperties, object::properties::ObjectProperties,
};
use actix_web::http::header;
use actix_web::{web, HttpResponse};
use chrono::{DateTime, FixedOffset, Utc};
use sqlx::Row;
use tracing::instrument;
use uuid::Uuid;

#[instrument(skip(state))]
pub async fn post_follow(
	state: web::Data<AppState>,
	body: Follow,
	subject_user_id: Uuid,
	username: &str,
) -> Result<HttpResponse, ApiError> {
	let follow_props: &FollowProperties = body.as_ref();
	let actor_url = follow_props
		.get_actor_xsd_any_uri()
		.ok_or(ApiError::OtherBadRequest)?;

	if let Some(captures) = url::user_url_regex().captures(actor_url.as_str()) {
		let captured_username = captures.get(1).unwrap().as_str();
		if captured_username != username {
			return Err(ApiError::OtherBadRequest);
		}
	} else {
		return Err(ApiError::OtherBadRequest);
	}

	let object_url = follow_props
		.get_object_xsd_any_uri()
		.ok_or(ApiError::OtherBadRequest)?;

	if let Some(captures) = url::user_url_regex().captures(object_url.as_str()) {
		let captured_username = captures.get(1).unwrap().as_str();

		let object_user_id: Option<Uuid> =
			sqlx::query("SELECT id FROM users WHERE username = $1 AND this_instance = TRUE")
				.bind(captured_username)
				.fetch_optional(&state.db)
				.await?
				.map(|row| row.get(0));
		if object_user_id.is_none() {
			return Err(ApiError::UserDoesNotExist);
		}
		let object_user_id = object_user_id.unwrap();

		let activity_id = Uuid::new_v4();
		let published_at = Utc::now();
		let published_at_fixed: DateTime<FixedOffset> = published_at.into();

		let mut new_activity = Follow::new();
		let object_props: &mut ObjectProperties = new_activity.as_mut();

		object_props.set_context_xsd_any_uri("https://www.w3.org/ns/activitystreams")?;
		object_props.set_id(url::activitypub_activity(activity_id))?;
		object_props.set_published(published_at_fixed)?;
		object_props
			.set_many_cc_xsd_any_uris(vec!["https://www.w3.org/ns/activitystreams#Public"])?;

		let follow_props: &mut FollowProperties = new_activity.as_mut();

		follow_props.set_actor_xsd_any_uri(url::activitypub_actor(username))?;
		follow_props.set_object_xsd_any_uri(url::activitypub_actor(captured_username))?;

		let empty_vec: Vec<Uuid> = Vec::new();

		let serialized_activity = serde_json::to_value(new_activity)?;

		let mut tx = state.db.begin().await?;

		sqlx::query("INSERT INTO activities (id, user_id, this_instance, published_at, activity, is_public, to_mentions, cc_mentions, to_followers_of, cc_followers_of) VALUES ($1, $2, TRUE, $3, $4, TRUE, $5, $6, $7, $8)")
			.bind(activity_id)
			.bind(subject_user_id)
			.bind(published_at.naive_utc())
			.bind(serialized_activity)
			.bind(&empty_vec)
			.bind(&empty_vec)
			.bind(&empty_vec)
			.bind(&empty_vec)
			.execute(&mut tx)
			.await?;

		sqlx::query("INSERT INTO follows (subject_user_id, object_user_id, following_since, pending) VALUES ($1, $2, $3, FALSE)")
			.bind(subject_user_id)
			.bind(object_user_id)
			.bind(published_at.naive_utc())
			.execute(&mut tx)
			.await?;

		tx.commit().await?;

		Ok(HttpResponse::Created()
			.insert_header((header::LOCATION, url::activitypub_activity(activity_id)))
			.finish())
	} else {
		todo!("discovering foreign users");
	}
}
