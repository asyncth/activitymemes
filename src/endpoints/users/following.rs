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

use crate::activitypub::collections::following::{Data, Following};
use crate::activitypub::collections::Collection;
use crate::error::ApiError;
use crate::state::AppState;
use activitystreams::collection::{OrderedCollection, OrderedCollectionPage};
use actix_web::{get, web, Either};
use serde::Deserialize;
use sqlx::Row;
use tracing::instrument;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct GetFollowingQuery {
	#[serde(default)]
	page: bool,
	max_id: Option<i64>,
	min_id: Option<i64>,
}

#[get("/{username}/following")]
#[instrument(skip(state))]
pub async fn get_following(
	state: web::Data<AppState>,
	path: web::Path<String>,
	query: web::Query<GetFollowingQuery>,
) -> Result<Either<web::Json<OrderedCollection>, web::Json<OrderedCollectionPage>>, ApiError> {
	let username = path.into_inner();

	let user_id: Option<Uuid> =
		sqlx::query("SELECT id FROM users WHERE username = $1 AND this_instance = TRUE")
			.bind(&username)
			.fetch_optional(&state.db)
			.await?
			.map(|row| row.get(0));
	if user_id.is_none() {
		return Err(ApiError::UserDoesNotExist);
	}
	let user_id = user_id.unwrap();

	let collection = Collection::new(Following::new(state.clone()));
	let data = Data { user_id, username };

	if query.page {
		if query.max_id.is_none() && query.min_id.is_none() {
			return collection
				.first_page(&data)
				.await
				.map(|val| Either::Right(web::Json(val)));
		}

		if query.max_id.is_some() && query.min_id.is_some() {
			return Err(ApiError::OtherBadRequest);
		}

		if let Some(max_id) = query.max_id {
			return collection
				.max_id_page(max_id, &data)
				.await
				.map(|val| Either::Right(web::Json(val)));
		}

		if let Some(min_id) = query.min_id {
			return collection
				.min_id_page(min_id, &data)
				.await
				.map(|val| Either::Right(web::Json(val)));
		}
	}

	collection
		.index_page(&data)
		.await
		.map(|val| Either::Left(web::Json(val)))
}
