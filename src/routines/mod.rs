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

pub mod delivery;

pub use delivery::deliver_activity;

use crate::error::ApiError;
use crate::state::AppState;
use activitystreams::actor::properties::ApActorProperties;
use activitystreams::actor::Person;
use activitystreams::ext::Ext;
use activitystreams::object::properties::ObjectProperties;
use activitystreams::BaseBox;
use awc::http::header;
use awc::Client;
use sqlx::Row;
use std::time::Duration;
use tracing::instrument;
use url::Url;
use uuid::Uuid;

thread_local! {
	static CLIENT: Client = Client::builder().timeout(Duration::from_secs(10)).finish();
}

#[instrument(skip(state))]
pub async fn fetch_remote_actor(state: &AppState, actor_id: &Url) -> Result<Uuid, ApiError> {
	let user_id: Option<Uuid> =
		sqlx::query("SELECT id FROM users WHERE this_instance = FALSE and instance_url = $1")
			.bind(actor_id.as_str())
			.map(|row| row.get(0))
			.fetch_optional(&state.db)
			.await?;
	if let Some(id) = user_id {
		return Ok(id);
	}

	if actor_id.scheme() != "https" {
		return Err(ApiError::OtherBadRequest);
	}

	let request = CLIENT.with(|client| {
		client
			.get(actor_id.as_str())
			.insert_header((
				header::ACCEPT,
				"application/ld+json; profile=\"https://www.w3.org/ns/activitystreams\"",
			))
			.send()
	});

	let mut response = request.await?;
	let body = response.body().await?;
	let actor: BaseBox = serde_json::from_slice(&body)?;

	match actor.kind() {
		Some("Person") => {
			let actor: Ext<Person, ApActorProperties> = actor
				.into_concrete()
				.map_err(|_| ApiError::UnexpectedResponseFromFederatedServer)?;
			let object_props: &ObjectProperties = actor.as_ref();

			// Check if we just accidentally sent a request to ourselves.
			// This can happen if sanitizer didn't recognize the URL pointing to
			// because an IP address was passed instead of a domain.
			if let Some(id) = object_props.get_id() {
				let url = Url::parse(id.as_str())?;
				if let Some(domain) = url.domain() {
					if domain == state.domain {
						return Err(ApiError::OtherBadRequest);
					}
				}
			}

			let name = object_props
				.get_name_xsd_string()
				.map(|xsd_string| xsd_string.as_str());
			let summary = object_props
				.get_summary_xsd_string()
				.map(|xsd_string| xsd_string.as_str());

			let ap_actor_props = &actor.extension;
			let username = ap_actor_props
				.get_preferred_username()
				.ok_or(ApiError::UnexpectedResponseFromFederatedServer)?
				.as_str();

			let id = Uuid::new_v4();
			sqlx::query("INSERT INTO users (id, username, this_instance, instance_url, name, bio) VALUES ($1, $2, FALSE, $3, $4, $5)")
				.bind(id)
				.bind(username)
				.bind(actor_id.as_str())
				.bind(name)
				.bind(summary)
				.execute(&state.db)
				.await?;

			Ok(id)
		}
		Some(_) => todo!("discovering non-person actors"),
		None => Err(ApiError::UnexpectedResponseFromFederatedServer),
	}
}
