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

use super::{Item, Items, Provider};
use crate::error::ApiError;
use futures::Stream as FuturesStream;
use pin_project::pin_project;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tracing::error;

#[pin_project]
pub struct Stream<'a, T>
where
	T: Provider,
	<T as Provider>::Data: Debug,
{
	fut: Pin<Box<dyn Future<Output = Result<Items, <T as Provider>::Error>> + 'a>>,
	provider: &'a T,
	data: &'a <T as Provider>::Data,
	items: VecDeque<Item>,
	max_id: Option<i64>,
	failed: bool,
}

impl<'a, T> Stream<'a, T>
where
	T: Provider,
	<T as Provider>::Data: Debug,
{
	pub fn new(provider: &'a T, data: &'a <T as Provider>::Data) -> Self {
		Self {
			fut: provider.fetch_first_page(data),
			provider,
			data,
			items: VecDeque::new(),
			max_id: None,
			failed: false,
		}
	}
}

impl<T> FuturesStream for Stream<'_, T>
where
	T: Provider,
	<T as Provider>::Data: Debug,
{
	type Item = Result<Item, ApiError>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let this = self.project();

		if *this.failed {
			return Poll::Ready(None);
		}

		if !this.items.is_empty() {
			return Poll::Ready(Some(Ok(this.items.pop_front().unwrap())));
		}

		if let Some(max_id) = *this.max_id {
			*this.fut = this.provider.fetch_max_id(max_id, this.data);
		}

		match this.fut.as_mut().poll(cx) {
			Poll::Ready(result) => match result {
				Ok(items) => {
					let items: Vec<Item> = match items {
						Items::BaseBox(v) => v.into_iter().map(Item::BaseBox).collect(),
						Items::XsdString(v) => v.into_iter().map(Item::XsdString).collect(),
					};

					if items.is_empty() {
						Poll::Ready(None)
					} else {
						let last_item = items.last().unwrap();
						let last_id = match last_item {
							Item::BaseBox(item) => item.id,
							Item::XsdString(item) => item.id,
						};

						*this.max_id = Some(last_id);
						*this.items = items.into();
						Poll::Ready(Some(Ok(this.items.pop_front().unwrap())))
					}
				}
				Err(err) => {
					*this.failed = true;

					error!(?err, "Failed to get the next page");
					Poll::Ready(Some(Err(ApiError::InternalServerError)))
				}
			},
			Poll::Pending => Poll::Pending,
		}
	}
}
