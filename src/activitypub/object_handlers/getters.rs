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

use activitystreams::activity::properties::ActorAndObjectProperties;
use activitystreams::primitives::XsdAnyUri;
use activitystreams::{object::properties::ObjectProperties, BaseBox};

pub fn get_name<T>(obj: &T) -> Option<&str>
where
	T: AsRef<ObjectProperties>,
{
	let object_props = obj.as_ref();
	let name = object_props.get_name_xsd_string()?;

	Some(name.as_str())
}

pub fn get_summary<T>(obj: &T) -> Option<&str>
where
	T: AsRef<ObjectProperties>,
{
	let object_props = obj.as_ref();
	let summary = object_props.get_summary_xsd_string()?;

	Some(summary.as_str())
}

pub fn get_url<T>(obj: &T) -> Option<&XsdAnyUri>
where
	T: AsRef<ObjectProperties>,
{
	let object_props = obj.as_ref();
	let url = object_props.get_url_xsd_any_uri()?;

	Some(url)
}

pub fn get_to<T>(obj: &T) -> Option<Vec<&XsdAnyUri>>
where
	T: AsRef<ObjectProperties>,
{
	let object_props = obj.as_ref();
	let to: Vec<&XsdAnyUri> = object_props.get_many_to_xsd_any_uris()?.collect();

	Some(to)
}

pub fn get_cc<T>(obj: &T) -> Option<Vec<&XsdAnyUri>>
where
	T: AsRef<ObjectProperties>,
{
	let object_props = obj.as_ref();
	let cc: Vec<&XsdAnyUri> = object_props.get_many_cc_xsd_any_uris()?.collect();

	Some(cc)
}

pub fn get_object_base_box<T>(obj: &T) -> Option<&BaseBox>
where
	T: AsRef<ActorAndObjectProperties>,
{
	let actor_and_object_props = obj.as_ref();
	let object = actor_and_object_props.get_object_base_box()?;

	Some(object)
}

pub fn get_object_xsd_any_uri<T>(obj: &T) -> Option<&XsdAnyUri>
where
	T: AsRef<ActorAndObjectProperties>,
{
	let actor_and_object_props = obj.as_ref();
	let object = actor_and_object_props.get_object_xsd_any_uri()?;

	Some(object)
}

pub fn get_actor_xsd_any_uri<T>(obj: &T) -> Option<&XsdAnyUri>
where
	T: AsRef<ActorAndObjectProperties>,
{
	let actor_and_object_props = obj.as_ref();
	let actor = actor_and_object_props.get_actor_xsd_any_uri()?;

	Some(actor)
}
