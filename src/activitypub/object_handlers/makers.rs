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

// TODO: Fix this issue.
#![allow(clippy::unnecessary_unwrap)]

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
use std::str::FromStr;
use uuid::Uuid;

// TODO: Move common activity args into a separate struct and use that instead.
#[allow(clippy::too_many_arguments)]
pub fn new_image(
	activity_id: Uuid,
	actor_url: XsdAnyUri,
	name: &str,
	summary: Option<&str>,
	image_url: XsdAnyUri,
	published_at: DateTime<Utc>,
	to: Option<Vec<XsdAnyUri>>,
	cc: Option<Vec<XsdAnyUri>>,
) -> Result<Image, ApiError> {
	let mut image = Image::new();
	let object_props: &mut ObjectProperties = image.as_mut();

	object_props.set_context_xsd_any_uri("https://www.w3.org/ns/activitystreams")?;
	object_props.set_id(url::activitypub_object(activity_id))?;
	object_props.set_name_xsd_string(name.trim())?;
	object_props.set_url_xsd_any_uri(image_url)?;
	object_props.set_attributed_to_xsd_any_uri(actor_url)?;
	object_props.set_published(DateTime::<FixedOffset>::from(published_at))?;

	if let Some(summary) = summary {
		object_props.set_summary_xsd_string(summary.trim())?;
	}

	if to.is_none() && cc.is_none() {
		return Err(ApiError::OtherBadRequest);
	} else if to.is_some() && cc.is_none() {
		let to = to.unwrap();
		if to.is_empty() {
			return Err(ApiError::OtherBadRequest);
		}

		object_props.set_many_to_xsd_any_uris(to)?;
	} else if to.is_none() && cc.is_some() {
		let cc = cc.unwrap();
		if cc.is_empty() {
			return Err(ApiError::OtherBadRequest);
		}

		object_props.set_many_cc_xsd_any_uris(cc)?;
	} else {
		let to = to.unwrap();
		let cc = cc.unwrap();

		if to.is_empty() && cc.is_empty() {
			return Err(ApiError::OtherBadRequest);
		}

		object_props.set_many_to_xsd_any_uris(to)?;
		object_props.set_many_cc_xsd_any_uris(cc)?;
	}

	Ok(image)
}

pub fn new_create(
	activity_id: Uuid,
	actor_url: XsdAnyUri,
	published_at: DateTime<Utc>,
	inner_object: BaseBox,
	to: Option<Vec<XsdAnyUri>>,
	cc: Option<Vec<XsdAnyUri>>,
) -> Result<Create, ApiError> {
	let mut create = Create::new();
	let object_props: &mut ObjectProperties = create.as_mut();

	object_props.set_context_xsd_any_uri("https://www.w3.org/ns/activitystreams")?;
	object_props.set_id(url::activitypub_activity(activity_id))?;
	object_props.set_published(DateTime::<FixedOffset>::from(published_at))?;

	if to.is_none() && cc.is_none() {
		return Err(ApiError::OtherBadRequest);
	} else if to.is_some() && cc.is_none() {
		let to = to.unwrap();
		if to.is_empty() {
			return Err(ApiError::OtherBadRequest);
		}

		object_props.set_many_to_xsd_any_uris(to)?;
	} else if to.is_none() && cc.is_some() {
		let cc = cc.unwrap();
		if cc.is_empty() {
			return Err(ApiError::OtherBadRequest);
		}

		object_props.set_many_cc_xsd_any_uris(cc)?;
	} else {
		let to = to.unwrap();
		let cc = cc.unwrap();

		if to.is_empty() && cc.is_empty() {
			return Err(ApiError::OtherBadRequest);
		}

		object_props.set_many_to_xsd_any_uris(to)?;
		object_props.set_many_cc_xsd_any_uris(cc)?;
	}

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
) -> Result<Follow, ApiError> {
	let mut follow = Follow::new();
	let object_props: &mut ObjectProperties = follow.as_mut();

	object_props.set_context_xsd_any_uri("https://www.w3.org/ns/activitystreams")?;
	object_props.set_id(url::activitypub_activity(activity_id))?;
	object_props.set_published(DateTime::<FixedOffset>::from(published_at))?;
	object_props.set_many_to_xsd_any_uris(Vec::<XsdAnyUri>::new())?;
	object_props.set_many_cc_xsd_any_uris(vec![XsdAnyUri::from_str(
		"https://www.w3.org/ns/activitystreams#Public",
	)?])?;

	let actor_and_object_props: &mut ActorAndObjectProperties = follow.as_mut();

	actor_and_object_props.set_actor_xsd_any_uri(actor_url)?;
	actor_and_object_props.set_object_xsd_any_uri(object_url)?;

	Ok(follow)
}
