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

use crate::activitypub::outbox::post::object::UnsanitizedObject;
use crate::error::ApiError;
use crate::state::AppState;
use activitystreams::object::Image;
use actix_web::http::header;
use actix_web::{web, HttpResponse};
use tracing::instrument;
use uuid::Uuid;

#[instrument]
pub async fn post_image(
	state: web::Data<AppState>,
	body: Image,
	user_id: Uuid,
	username: String,
) -> Result<HttpResponse, ApiError> {
	let id = Uuid::new_v4();
	let shared_uri = format!("{}://{}/", state.scheme, state.domain);
	let activity_uri = format!("{}activities/{}", shared_uri, id);
	let object_uri = format!("{}/object", activity_uri);
	let actor_uri = format!("{}users/{}", shared_uri, username);

	let image = UnsanitizedObject::new(body).sanitize(&object_uri, Some(&actor_uri))?;
	let activity = image.activity(&activity_uri, &actor_uri)?;

	let serialized_activity =
		serde_json::to_value(activity).map_err(|_| ApiError::InternalServerError)?;

	// Needed because we don't support `to` and `cc` yet.
	let empty_vec: Vec<Uuid> = Vec::new();

	sqlx::query("INSERT INTO activities (id, this_instance, user_id, to_mentions, cc_mentions, is_public, activity) VALUES ($1, TRUE, $2, $3, $4, TRUE, $5)")
		.bind(id)
		.bind(user_id)
		.bind(&empty_vec)
		.bind(&empty_vec)
		.bind(serialized_activity)
		.execute(&state.db)
		.await?;

	Ok(HttpResponse::Created()
		.insert_header((header::LOCATION, activity_uri))
		.finish())
}
