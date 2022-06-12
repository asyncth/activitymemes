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

pub mod followers;
pub mod following;
pub mod inbox;
pub mod outbox;

pub use followers::get_followers;
pub use following::get_following;
pub use inbox::get_inbox;
pub use inbox::post_inbox;
pub use outbox::get_outbox;
pub use outbox::post_outbox;

use crate::error::ApiError;
use crate::state::AppState;
use crate::url;
use activitystreams::actor::properties::ApActorProperties;
use activitystreams::actor::Person;
use activitystreams::ext::Extensible;
use actix_web::{get, web};
use sqlx::Row;
use std::collections::HashMap;
use tracing::instrument;

#[get("/{username}")]
#[instrument(skip(state))]
pub async fn get_user(
	state: web::Data<AppState>,
	path: web::Path<String>,
) -> Result<web::Json<serde_json::Value>, ApiError> {
	let username = path.into_inner();

	let row =
		sqlx::query("SELECT name, bio, profile_picture_id, public_key FROM users WHERE username = $1 AND this_instance = TRUE")
			.bind(&username)
			.fetch_optional(&state.db)
			.await?;
	if row.is_none() {
		return Err(ApiError::UserDoesNotExist);
	}
	let row = row.unwrap();

	let mut user = Person::new().extend(ApActorProperties::default());
	let user_props = user.as_mut();
	let actor_url = url::activitypub_actor(&username);

	user_props.set_context_xsd_any_uri("https://www.w3.org/ns/activitystreams")?;
	user_props.set_id(&*actor_url)?;

	let name: &str = row.get(0);
	user_props.set_name_xsd_string(name)?;

	let bio: Option<&str> = row.get(1);
	if let Some(bio) = bio {
		user_props.set_summary_xsd_string(bio)?;
	}

	let profile_picture_id: Option<&str> = row.get(2);
	if let Some(profile_picture_id) = profile_picture_id {
		user_props.set_icon_xsd_any_uri(format!(
			"{}://{}/media/{}",
			state.scheme, state.domain, profile_picture_id
		))?;
	}

	let public_key: &str = row.get(3);

	let user_ap_props = &mut user.extension;

	user_ap_props.set_preferred_username(username.clone())?;
	user_ap_props.set_inbox(format!("{}/inbox", &actor_url))?;
	user_ap_props.set_outbox(format!("{}/outbox", &actor_url))?;
	user_ap_props.set_following(format!("{}/following", &actor_url))?;
	user_ap_props.set_followers(format!("{}/followers", &actor_url))?;

	let serialized_data = serde_json::to_value(user)?;
	let mut deserialized_data: HashMap<String, serde_json::Value> =
		serde_json::from_value(serialized_data)?;
	let user_url = url::activitypub_actor(&username);

	deserialized_data.insert(
		"publicKey".to_string(),
		serde_json::json!({
			"id": format!("{}#main-key", user_url),
			"owner": user_url,
			"publicKeyPem": public_key
		}),
	);

	let serialized_data = serde_json::to_value(deserialized_data)?;

	Ok(web::Json(serialized_data))
}
