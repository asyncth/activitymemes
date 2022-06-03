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
use activitystreams::actor::properties::ApActorProperties;
use activitystreams::actor::Person;
use activitystreams::ext::{Ext, Extensible};
use actix_web::{get, web};
use sqlx::Row;
use tracing::instrument;

#[get("/users/{username}")]
#[instrument]
pub async fn get_user(
	state: web::Data<AppState>,
	path: web::Path<String>,
) -> Result<web::Json<Ext<Person, ApActorProperties>>, ApiError> {
	let username = path.into_inner();

	let row =
		sqlx::query("SELECT name, bio, profile_picture_id FROM users WHERE username = $1 AND this_instance = TRUE")
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

	let user_ap_props = &mut user.extension;

	user_ap_props.set_preferred_username(username)?;
	user_ap_props.set_inbox(format!("{}/inbox", &actor_url))?;
	user_ap_props.set_outbox(format!("{}/outbox", &actor_url))?;
	user_ap_props.set_following(format!("{}/following", &actor_url))?;
	user_ap_props.set_followers(format!("{}/followers", &actor_url))?;

	Ok(web::Json(user))
}
