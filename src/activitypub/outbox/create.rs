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

use crate::activitypub::outbox::object::UnsanitizedObject;
use crate::error::ApiError;
use crate::state::AppState;
use crate::url;
use activitystreams::activity::properties::{ActivityProperties, CreateProperties};
use activitystreams::activity::Create;
use activitystreams::object::kind::{ImageType, NoteType};
use activitystreams::object::properties::ObjectProperties;
use activitystreams::object::{Image, Note};
use activitystreams::BaseBox;
use actix_web::http::header;
use actix_web::HttpResponse;
use tracing::instrument;
use uuid::Uuid;

struct UnsanitizedCreate {
	obj: Create,
}

impl UnsanitizedCreate {
	fn new(obj: Create) -> Self {
		Self { obj }
	}

	fn sanitize(self, activity_uri: &str, actor_url: &str) -> Result<SanitizedCreate, ApiError> {
		let mut activity = UnsanitizedObject::new(self.obj)
			.sanitize(activity_uri, None)?
			.into_inner();

		let create_props: &mut CreateProperties = activity.as_mut();
		let published = if let Some(obj) = create_props.get_object_base_box() {
			if obj.is_kind(ImageType) {
				let internal_object: Image = obj.clone().into_concrete()?;
				let internal_object = UnsanitizedObject::new(internal_object)
					.sanitize(&format!("{}/object", activity_uri), Some(actor_url))?
					.into_inner();

				let internal_object_props: &ObjectProperties = internal_object.as_ref();
				let published = internal_object_props.get_published().cloned().unwrap();

				let internal_object: BaseBox = internal_object.try_into()?;
				create_props.set_object_base_box(internal_object)?;

				published
			} else if obj.is_kind(NoteType) {
				let internal_object: Note = obj.clone().into_concrete()?;
				let internal_object = UnsanitizedObject::new(internal_object)
					.sanitize(&format!("{}/object", activity_uri), Some(actor_url))?
					.into_inner();

				let internal_object_props: &ObjectProperties = internal_object.as_ref();
				let published = internal_object_props.get_published().cloned().unwrap();

				let internal_object: BaseBox = internal_object.try_into()?;
				create_props.set_object_base_box(internal_object)?;

				published
			} else {
				return Err(ApiError::OtherBadRequest);
			}
		} else {
			return Err(ApiError::OtherBadRequest);
		};

		create_props.set_actor_xsd_any_uri(actor_url)?;

		let create_activity_props: &mut ActivityProperties = activity.as_mut();
		create_activity_props.delete_instrument();
		create_activity_props.delete_result();

		let create_object_props: &mut ObjectProperties = activity.as_mut();
		create_object_props.set_published(published)?;

		Ok(SanitizedCreate { obj: activity })
	}
}

struct SanitizedCreate {
	obj: Create,
}

impl SanitizedCreate {
	fn into_inner(self) -> Create {
		self.obj
	}
}

#[instrument]
pub async fn post_create(
	state: &AppState,
	body: Create,
	user_id: Uuid,
	username: &str,
) -> Result<HttpResponse, ApiError> {
	let id = Uuid::new_v4();
	let activity_url = url::activitypub_activity(id);
	let actor_url = url::activitypub_actor(username);

	let activity = UnsanitizedCreate::new(body)
		.sanitize(&activity_url, &actor_url)?
		.into_inner();

	let published_at = activity
		.object_props
		.get_published()
		.unwrap()
		.as_datetime()
		.naive_utc();
	let serialized_activity = serde_json::to_value(activity)?;

	// Needed because we don't support `to` and `cc` yet.
	let empty_vec: Vec<Uuid> = Vec::new();

	sqlx::query("INSERT INTO activities (id, user_id, this_instance, published_at, activity, is_public, to_mentions, cc_mentions) VALUES ($1, $2, TRUE, $3, $4, TRUE, $5, $6)")
		.bind(id)
		.bind(user_id)
		.bind(published_at)
		.bind(serialized_activity)
		.bind(&empty_vec)
		.bind(&empty_vec)
		.execute(&state.db)
		.await?;

	Ok(HttpResponse::Created()
		.insert_header((header::LOCATION, activity_url))
		.finish())
}
