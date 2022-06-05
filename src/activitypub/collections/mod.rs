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

use crate::error::ApiError;
use activitystreams::collection::properties::{CollectionPageProperties, CollectionProperties};
use activitystreams::collection::{OrderedCollection, OrderedCollectionPage};
use activitystreams::object::properties::ObjectProperties;
use activitystreams::BaseBox;
use async_trait::async_trait;
use std::borrow::Cow;
use std::error::Error as StdError;
use std::fmt::Debug;
use tracing::{error, instrument};

#[derive(Clone, Debug)]
pub struct ItemBaseBox {
	id: i64,
	data: BaseBox,
}

#[derive(Clone, Debug)]
pub struct ItemXsdString {
	id: i64,
	data: String,
}

#[derive(Clone, Debug)]
pub enum Items {
	BaseBox(Vec<ItemBaseBox>),
	XsdString(Vec<ItemXsdString>),
}

impl Items {
	fn len(&self) -> usize {
		match self {
			Self::BaseBox(v) => v.len(),
			Self::XsdString(v) => v.len(),
		}
	}

	fn is_empty(&self) -> bool {
		match self {
			Self::BaseBox(v) => v.is_empty(),
			Self::XsdString(v) => v.is_empty(),
		}
	}

	fn first_id(&self) -> Option<i64> {
		match self {
			Self::BaseBox(v) => v.first().map(|val| val.id),
			Self::XsdString(v) => v.first().map(|val| val.id),
		}
	}

	fn last_id(&self) -> Option<i64> {
		match self {
			Self::BaseBox(v) => v.last().map(|val| val.id),
			Self::XsdString(v) => v.last().map(|val| val.id),
		}
	}
}

#[async_trait(?Send)]
pub trait Provider {
	type Error: StdError;
	type Data;

	fn activitypub_id<'a>(&'a self, data: &'a Self::Data) -> Cow<'a, str>;
	async fn total_items(&self, data: &Self::Data) -> Result<u64, Self::Error>;
	async fn fetch_first_page(&self, data: &Self::Data) -> Result<Items, Self::Error>;
	async fn fetch_max_id(&self, max_id: i64, data: &Self::Data) -> Result<Items, Self::Error>;
	async fn fetch_min_id(&self, min_id: i64, data: &Self::Data) -> Result<Items, Self::Error>;
}

#[derive(Debug)]
pub struct Collection<T>
where
	T: Provider + Debug,
{
	provider: T,
}

impl<T> Collection<T>
where
	T: Provider + Debug,
	<T as Provider>::Data: Debug,
{
	pub fn new(provider: T) -> Self {
		Self { provider }
	}

	#[instrument]
	pub fn id<'a>(&'a self, data: &'a <T as Provider>::Data) -> Cow<'a, str> {
		self.provider.activitypub_id(data)
	}

	#[instrument]
	pub async fn len(&self, data: &<T as Provider>::Data) -> Result<u64, ApiError> {
		self.provider.total_items(data).await.map_err(|err| {
			error!(?err, "Failed to get the length of a collection");
			ApiError::InternalServerError
		})
	}

	#[instrument]
	pub async fn index_page(
		&self,
		data: &<T as Provider>::Data,
	) -> Result<OrderedCollection, ApiError> {
		let id = self.id(data);

		let mut collection = OrderedCollection::new();
		let collection_object_props: &mut ObjectProperties = collection.as_mut();

		collection_object_props.set_context_xsd_any_uri("https://www.w3.org/ns/activitystreams")?;
		collection_object_props.set_id(&*id)?;

		let collection_props: &mut CollectionProperties = collection.as_mut();

		collection_props.set_total_items(self.len(data).await?)?;
		collection_props.set_first_xsd_any_uri(format!("{}/inbox?page=true", id))?;
		collection_props.set_last_xsd_any_uri(format!("{}/inbox?min_id=0&page=true", id))?;

		Ok(collection)
	}

	#[instrument]
	pub async fn first_page(
		&self,
		data: &<T as Provider>::Data,
	) -> Result<OrderedCollectionPage, ApiError> {
		let (part_of, mut page) = self.prepare_page(data)?;
		let items = self.provider.fetch_first_page(data).await.map_err(|err| {
			error!(?err, "Failed to fetch the first page of a collection");
			ApiError::InternalServerError
		})?;

		self.add_next_prev_and_finalize(&mut page, items, &part_of, false)?;
		Ok(page)
	}

	#[instrument]
	pub async fn max_id_page(
		&self,
		max_id: i64,
		data: &<T as Provider>::Data,
	) -> Result<OrderedCollectionPage, ApiError> {
		let (part_of, mut page) = self.prepare_page(data)?;
		let items = self
			.provider
			.fetch_max_id(max_id, data)
			.await
			.map_err(|err| {
				error!(?err, "Failed to fetch max_id page of a collection");
				ApiError::InternalServerError
			})?;

		self.add_next_prev_and_finalize(&mut page, items, &part_of, true)?;
		Ok(page)
	}

	#[instrument]
	pub async fn min_id_page(
		&self,
		min_id: i64,
		data: &<T as Provider>::Data,
	) -> Result<OrderedCollectionPage, ApiError> {
		let (part_of, mut page) = self.prepare_page(data)?;
		let items = self
			.provider
			.fetch_min_id(min_id, data)
			.await
			.map_err(|err| {
				error!(?err, "Failed to fetch min_id page of a collection");
				ApiError::InternalServerError
			})?;

		self.add_next_prev_and_finalize(&mut page, items, &part_of, true)?;
		Ok(page)
	}

	#[instrument]
	fn prepare_page<'a>(
		&'a self,
		data: &'a <T as Provider>::Data,
	) -> Result<(Cow<'a, str>, OrderedCollectionPage), ApiError> {
		let part_of = self.id(data);

		let mut page = OrderedCollectionPage::new();
		let page_object_props: &mut ObjectProperties = page.as_mut();

		page_object_props.set_context_xsd_any_uri("https://www.w3.org/ns/activitystreams")?;
		page_object_props.set_id(format!("{}?page=true", part_of))?;

		let page_props: &mut CollectionPageProperties = page.as_mut();
		page_props.set_part_of_xsd_any_uri(&*part_of)?;

		Ok((part_of, page))
	}

	#[instrument(skip(page, items))]
	fn add_next_prev_and_finalize(
		&self,
		page: &mut OrderedCollectionPage,
		items: Items,
		part_of: &str,
		do_add_prev_page: bool,
	) -> Result<(), ApiError> {
		if !items.is_empty() {
			let last_id = items.last_id().unwrap();

			if do_add_prev_page {
				let page_props: &mut CollectionPageProperties = page.as_mut();
				let first_id = items.first_id().unwrap();
				page_props
					.set_prev_xsd_any_uri(format!("{}?min_id={}&page=true", part_of, first_id))?;
			}
			if items.len() == 20 {
				let page_props: &mut CollectionPageProperties = page.as_mut();
				page_props
					.set_next_xsd_any_uri(format!("{}?max_id={}&page=true", part_of, last_id))?;
			}

			let page_props: &mut CollectionProperties = page.as_mut();
			match items {
				Items::BaseBox(v) => {
					let v: Vec<BaseBox> = v.into_iter().map(|val| val.data).collect();
					page_props.set_many_items_base_boxes(v)?;
				}
				Items::XsdString(v) => {
					let v: Vec<String> = v.into_iter().map(|val| val.data).collect();
					page_props.set_many_items_xsd_strings(v)?;
				}
			}
		}

		Ok(())
	}
}
