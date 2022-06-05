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
use activitystreams::activity::properties::CreateProperties;
use activitystreams::activity::Create;
use activitystreams::object::properties::ObjectProperties;
use activitystreams::object::Object;
use activitystreams::BaseBox;
use chrono::{DateTime, FixedOffset, Utc};

pub struct UnsanitizedObject<T>
where
	T: Object + AsRef<ObjectProperties> + AsMut<ObjectProperties> + TryInto<BaseBox>,
{
	obj: T,
}

impl<T> UnsanitizedObject<T>
where
	T: Object + AsRef<ObjectProperties> + AsMut<ObjectProperties> + TryInto<BaseBox>,
{
	pub fn new(obj: T) -> Self {
		Self { obj }
	}

	pub fn sanitize(
		mut self,
		object_uri: &str,
		actor_uri: Option<&str>,
	) -> Result<SanitizedObject<T>, ApiError> {
		let object_props: &mut ObjectProperties = self.obj.as_mut();

		object_props.set_context_xsd_any_uri("https://www.w3.org/ns/activitystreams")?;

		object_props.delete_attachment();

		if let Some(actor_uri) = actor_uri {
			object_props.set_attributed_to_xsd_any_uri(actor_uri)?;
		}

		if let Some(name) = object_props.get_name_xsd_string() {
			let name = name.to_string();
			let name = name.trim();

			if name.is_empty() {
				return Err(ApiError::OtherBadRequest);
			}

			object_props.set_name_xsd_string(name)?;
		} else {
			return Err(ApiError::OtherBadRequest);
		}

		object_props.end_time = None;
		object_props.delete_generator();
		object_props.delete_icon();

		// `image` property is not where we actually store the URL of the image, we store it in `url` property.
		object_props.delete_image();

		// Currently reply objects are not allowed.
		object_props.delete_in_reply_to();
		object_props.delete_location();
		object_props.delete_preview();

		object_props.delete_replies();
		object_props.start_time = None;

		// `summary` property is used for alt text.
		if object_props.summary.is_some() {
			if let Some(summary) = object_props.get_summary_xsd_string() {
				let summary = summary.to_string();
				let summary = summary.trim();

				object_props.set_summary_xsd_string(summary)?;
			} else {
				return Err(ApiError::OtherBadRequest);
			}
		}

		object_props.delete_tag();
		object_props.updated = None;

		// TODO: Implement XsdAnyUri to `Link` object conversion with image type lookup.
		if object_props.url.is_none() {
			return Err(ApiError::OtherBadRequest);
		}

		let to = match object_props.get_many_to_xsd_any_uris() {
			Some(i) => i.take(5).cloned().collect(),
			None => match object_props.get_to_xsd_any_uri() {
				Some(one) => vec![one.clone()],
				None => Vec::new(),
			},
		};

		let cc = match object_props.get_many_cc_xsd_any_uris() {
			Some(i) => i.take(5).cloned().collect(),
			None => match object_props.get_cc_xsd_any_uri() {
				Some(one) => vec![one.clone()],
				None => Vec::new(),
			},
		};

		// At very least objects should be addressed to public.
		if to.is_empty() && cc.is_empty() {
			return Err(ApiError::OtherBadRequest);
		}

		// Make sure that these two are always set to something, even if it's an empty Vec.
		object_props.set_many_to_xsd_any_uris(to)?;
		object_props.set_many_cc_xsd_any_uris(cc)?;

		object_props.delete_bto();
		object_props.delete_bcc();
		object_props.media_type = None;
		object_props.duration = None;

		object_props.set_id(object_uri)?;
		object_props.set_published(DateTime::<FixedOffset>::from(Utc::now()))?;

		Ok(SanitizedObject { obj: self.obj })
	}
}

pub struct SanitizedObject<T>
where
	T: Object + AsRef<ObjectProperties> + AsMut<ObjectProperties> + TryInto<BaseBox>,
{
	obj: T,
}

impl<T> SanitizedObject<T>
where
	T: Object + AsRef<ObjectProperties> + AsMut<ObjectProperties> + TryInto<BaseBox>,
{
	pub fn into_inner(self) -> T {
		self.obj
	}

	pub fn activity(self, activity_uri: &str, actor_uri: &str) -> Result<Create, ApiError> {
		let object_props: &ObjectProperties = self.obj.as_ref();

		let published = object_props
			.get_published()
			.cloned()
			.expect("expected the object to have `published` field");

		let mut activity = Create::new();

		let create_object_props: &mut ObjectProperties = activity.as_mut();
		create_object_props.set_context_xsd_any_uri("https://www.w3.org/ns/activitystreams")?;
		create_object_props.set_id(activity_uri)?;
		create_object_props.set_published(published)?;
		create_object_props.set_many_to_xsd_any_uris(
			object_props
				.get_many_to_xsd_any_uris()
				.ok_or(ApiError::InternalServerError)
				.unwrap()
				.cloned()
				.collect(),
		)?;
		create_object_props.set_many_cc_xsd_any_uris(
			object_props
				.get_many_cc_xsd_any_uris()
				.ok_or(ApiError::InternalServerError)
				.unwrap()
				.cloned()
				.collect(),
		)?;

		let create_props: &mut CreateProperties = activity.as_mut();
		create_props.set_actor_xsd_any_uri(actor_uri)?;
		create_props.set_object_base_box(self.obj).map_err(|_| {
			// This error doesn't implement `Display` or `Debug`.
			ApiError::InternalServerError
		})?;

		Ok(activity)
	}
}
