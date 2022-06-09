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
use crate::url;
use activitystreams::activity::properties::ActorAndObjectProperties;
use activitystreams::activity::Create;
use activitystreams::activity::{properties::CreateProperties, Follow};
use activitystreams::object::properties::ObjectProperties;
use activitystreams::object::Image;
use activitystreams::primitives::XsdAnyUri;
use activitystreams::BaseBox;
use chrono::{DateTime, FixedOffset, Utc};
use uuid::Uuid;

// TODO: Move common activity args into a separate struct and use that instead.
#[allow(clippy::too_many_arguments)]
pub fn new_image(
	activity_id: Uuid,
	actor_url: XsdAnyUri,
	name: &str,
	summary: &str,
	image_url: XsdAnyUri,
	published_at: DateTime<Utc>,
	to: Vec<XsdAnyUri>,
	cc: Vec<XsdAnyUri>,
) -> Result<Image, ApiError> {
	let mut image = Image::new();
	let object_props: &mut ObjectProperties = image.as_mut();

	object_props.set_context_xsd_any_uri("https://www.w3.org/ns/activitystreams")?;
	object_props.set_id(url::activitypub_object(activity_id))?;
	object_props.set_name_xsd_string(name.trim())?;
	object_props.set_summary_xsd_string(summary.trim())?;
	object_props.set_url_xsd_any_uri(image_url)?;
	object_props.set_attributed_to_xsd_any_uri(actor_url)?;
	object_props.set_published(DateTime::<FixedOffset>::from(published_at))?;
	object_props.set_many_to_xsd_any_uris(to)?;
	object_props.set_many_cc_xsd_any_uris(cc)?;

	Ok(image)
}

pub fn new_create(
	activity_id: Uuid,
	actor_url: XsdAnyUri,
	published_at: DateTime<Utc>,
	inner_object: BaseBox,
	to: Vec<XsdAnyUri>,
	cc: Vec<XsdAnyUri>,
) -> Result<Create, ApiError> {
	let mut create = Create::new();
	let object_props: &mut ObjectProperties = create.as_mut();

	object_props.set_context_xsd_any_uri("https://www.w3.org/ns/activitystreams")?;
	object_props.set_id(url::activitypub_activity(activity_id))?;
	object_props.set_published(DateTime::<FixedOffset>::from(published_at))?;
	object_props.set_many_to_xsd_any_uris(to)?;
	object_props.set_many_cc_xsd_any_uris(cc)?;

	let create_props: &mut CreateProperties = create.as_mut();

	create_props.set_object_base_box(inner_object)?;
	create_props.set_actor_xsd_any_uri(actor_url)?;

	Ok(create)
}

pub fn new_follow(
	activity_id: Uuid,
	published_at: DateTime<Utc>,
	actor_url: XsdAnyUri,
	object_url: XsdAnyUri,
	to: Vec<XsdAnyUri>,
	cc: Vec<XsdAnyUri>,
) -> Result<Follow, ApiError> {
	let mut follow = Follow::new();
	let object_props: &mut ObjectProperties = follow.as_mut();

	object_props.set_context_xsd_any_uri("https://www.w3.org/ns/activitystreams")?;
	object_props.set_id(url::activitypub_activity(activity_id))?;
	object_props.set_published(DateTime::<FixedOffset>::from(published_at))?;
	object_props.set_many_to_xsd_any_uris(to)?;
	object_props.set_many_cc_xsd_any_uris(cc)?;

	let actor_and_object_props: &mut ActorAndObjectProperties = follow.as_mut();

	actor_and_object_props.set_actor_xsd_any_uri(actor_url)?;
	actor_and_object_props.set_object_xsd_any_uri(object_url)?;

	Ok(follow)
}
