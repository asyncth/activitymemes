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
use activitystreams::collection::properties::{CollectionPageProperties, CollectionProperties};
use activitystreams::collection::{OrderedCollection, OrderedCollectionPage};
use activitystreams::object::properties::ObjectProperties;
use actix_web::{get, web, Either};
use serde::Deserialize;
use sqlx::types::chrono::NaiveDateTime;
use sqlx::Row;
use std::convert::TryFrom;
use tracing::instrument;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize)]
pub struct GetFollowersQuery {
	page: Option<bool>,
	#[serde(rename = "max_id")]
	max_timestamp: Option<i64>,
	#[serde(rename = "min_id")]
	min_timestamp: Option<i64>,
}

#[get("/users/{username}/followers")]
#[instrument]
pub async fn get_followers(
	state: web::Data<AppState>,
	query: web::Query<GetFollowersQuery>,
	path: web::Path<String>,
) -> Result<Either<web::Json<OrderedCollection>, web::Json<OrderedCollectionPage>>, ApiError> {
	if let Some(page) = query.page {
		if page {
			return get_followers_page(state, query, path)
				.await
				.map(Either::Right);
		}
	}

	get_followers_index(state, path).await.map(Either::Left)
}

#[instrument]
async fn get_followers_index(
	state: web::Data<AppState>,
	path: web::Path<String>,
) -> Result<web::Json<OrderedCollection>, ApiError> {
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

	let mut collection = OrderedCollection::new();
	let collection_props: &mut ObjectProperties = collection.as_mut();

	collection_props.set_context_xsd_any_uri("https://www.w3.org/ns/activitystreams")?;

	let id_url = format!("{}/followers", url::activitypub_actor(&username));
	collection_props.set_id(&*id_url)?;

	let collection_props: &mut CollectionProperties = collection.as_mut();

	let total_items: i64 =
		sqlx::query("SELECT COUNT(1) FROM following WHERE following_user_id = $1")
			.bind(user_id)
			.fetch_one(&state.db)
			.await?
			.get(0);
	collection_props.set_total_items(u64::try_from(total_items)?)?;
	collection_props.set_first_xsd_any_uri(format!("{}?page=true", id_url))?;
	collection_props.set_last_xsd_any_uri(format!("{}?min_id=0&page=true", id_url))?;

	Ok(web::Json(collection))
}

async fn get_followers_page(
	state: web::Data<AppState>,
	query: web::Query<GetFollowersQuery>,
	path: web::Path<String>,
) -> Result<web::Json<OrderedCollectionPage>, ApiError> {
	if query.max_timestamp.is_some() && query.min_timestamp.is_some() {
		todo!("specifying both max_timestamp and min_timestamp");
	}

	if query.max_timestamp.is_some() {
		return get_followers_max_timestamp(state, query, path).await;
	} else if query.min_timestamp.is_some() {
		return get_followers_min_timestamp(state, query, path).await;
	}

	get_followers_first_page(state, path).await
}

#[instrument]
async fn get_followers_first_page(
	state: web::Data<AppState>,
	path: web::Path<String>,
) -> Result<web::Json<OrderedCollectionPage>, ApiError> {
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

	let mut page = OrderedCollectionPage::new();
	let page_props: &mut ObjectProperties = page.as_mut();

	page_props.set_context_xsd_any_uri("https://www.w3.org/ns/activitystreams")?;

	let part_of_id_url = format!("{}/followers", url::activitypub_actor(&username));
	page_props.set_id(format!("{}?page=true", part_of_id_url))?;

	let page_props: &mut CollectionPageProperties = page.as_mut();

	page_props.set_part_of_xsd_any_uri(part_of_id_url.as_str())?;

	let rows = sqlx::query(
        "SELECT following.inserted_at, users.this_instance, users.instance_url FROM following, users WHERE following.following_user_id = $1 AND following.following_user_id = users.id ORDER BY following.inserted_at DESC LIMIT 20",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

	if !rows.is_empty() {
		let first_timestamp: NaiveDateTime = rows[0].get(0);
		let last_timestamp: NaiveDateTime = rows[rows.len() - 1].get(0);

		let actors: Vec<String> = rows
			.iter()
			.map(|row| {
				let this_instance: bool = row.get(1);
				if this_instance {
					url::activitypub_actor(&username)
				} else {
					let instance_url: Option<String> = row.get(2);
					instance_url.unwrap()
				}
			})
			.collect();

		page_props.set_prev_xsd_any_uri(format!(
			"{}?min_id={}&page=true",
			part_of_id_url,
			first_timestamp.timestamp_millis()
		))?;
		if rows.len() == 20 {
			page_props.set_next_xsd_any_uri(format!(
				"{}?max_id={}&page=true",
				part_of_id_url,
				last_timestamp.timestamp_millis()
			))?;
		}

		let page_props: &mut CollectionProperties = page.as_mut();
		page_props.set_many_items_xsd_strings(actors)?;
	}

	Ok(web::Json(page))
}

#[instrument]
async fn get_followers_max_timestamp(
	state: web::Data<AppState>,
	query: web::Query<GetFollowersQuery>,
	path: web::Path<String>,
) -> Result<web::Json<OrderedCollectionPage>, ApiError> {
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

	let max_timestamp = query.max_timestamp.unwrap();

	let mut page = OrderedCollectionPage::new();
	let page_props: &mut ObjectProperties = page.as_mut();

	page_props.set_context_xsd_any_uri("https://www.w3.org/ns/activitystreams")?;

	let part_of_id_url = format!("{}/followers", url::activitypub_actor(&username));

	page_props.set_id(format!(
		"{}?max_id={}&page=true",
		part_of_id_url, max_timestamp
	))?;

	let page_props: &mut CollectionPageProperties = page.as_mut();

	page_props.set_part_of_xsd_any_uri(part_of_id_url.as_str())?;

	let rows = sqlx::query("SELECT following.inserted_at, users.this_instance, users.instance_url FROM following, users WHERE following.following_user_id = $1 AND following.inserted_at < $2 AND following.following_user_id = users.id ORDER BY following.inserted_at DESC LIMIT 20")
        .bind(user_id)
        .bind(max_timestamp)
        .fetch_all(&state.db)
        .await?;

	if !rows.is_empty() {
		let first_timestamp: NaiveDateTime = rows[0].get(0);
		let last_timestamp: NaiveDateTime = rows[rows.len() - 1].get(0);
		let actors: Vec<String> = rows
			.iter()
			.map(|row| {
				let this_instance: bool = row.get(1);
				if this_instance {
					url::activitypub_actor(&username)
				} else {
					let instance_url: Option<String> = row.get(2);
					instance_url.unwrap()
				}
			})
			.collect();

		page_props.set_prev_xsd_any_uri(format!(
			"{}?min_id={}&page=true",
			part_of_id_url,
			first_timestamp.timestamp_millis()
		))?;
		if rows.len() == 20 {
			page_props.set_next_xsd_any_uri(format!(
				"{}?max_id={}&page=true",
				part_of_id_url,
				last_timestamp.timestamp_millis()
			))?;
		}

		let page_props: &mut CollectionProperties = page.as_mut();
		page_props.set_many_items_xsd_strings(actors)?;
	}

	Ok(web::Json(page))
}

#[instrument]
async fn get_followers_min_timestamp(
	state: web::Data<AppState>,
	query: web::Query<GetFollowersQuery>,
	path: web::Path<String>,
) -> Result<web::Json<OrderedCollectionPage>, ApiError> {
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

	let min_timestamp = query.min_timestamp.unwrap();

	let mut page = OrderedCollectionPage::new();
	let page_props: &mut ObjectProperties = page.as_mut();

	page_props.set_context_xsd_any_uri("https://www.w3.org/ns/activitystreams")?;

	let part_of_id_url = format!("{}/followers", url::activitypub_actor(&username));

	page_props.set_id(format!(
		"{}?min_id={}&page=true",
		part_of_id_url, min_timestamp
	))?;

	let page_props: &mut CollectionPageProperties = page.as_mut();

	page_props.set_part_of_xsd_any_uri(part_of_id_url.as_str())?;

	let rows = sqlx::query("SELECT * FROM (SELECT following.inserted_at, users.this_instance, users.instance_url FROM following, users WHERE following.following_user_id = $1 AND following.inserted_at > $2 AND following.following_user_id = users.id ORDER BY following.inserted_at LIMIT 20) AS tmp ORDER BY inserted_at DESC")
        .bind(user_id)
        .bind(min_timestamp)
        .fetch_all(&state.db)
        .await?;

	if !rows.is_empty() {
		let first_timestamp: NaiveDateTime = rows[0].get(0);
		let last_timestamp: NaiveDateTime = rows[rows.len() - 1].get(0);
		let actors: Vec<String> = rows
			.iter()
			.map(|row| {
				let this_instance: bool = row.get(1);
				if this_instance {
					url::activitypub_actor(&username)
				} else {
					let instance_url: Option<String> = row.get(2);
					instance_url.unwrap()
				}
			})
			.collect();

		page_props.set_prev_xsd_any_uri(format!(
			"{}?min_id={}&page=true",
			part_of_id_url,
			first_timestamp.timestamp_millis()
		))?;
		if rows.len() == 20 {
			page_props.set_next_xsd_any_uri(format!(
				"{}?max_id={}&page=true",
				part_of_id_url,
				last_timestamp.timestamp_millis()
			))?;
		}

		let page_props: &mut CollectionProperties = page.as_mut();
		page_props.set_many_items_xsd_strings(actors)?;
	}

	Ok(web::Json(page))
}
