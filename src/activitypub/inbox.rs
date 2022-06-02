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
use crate::{account, url};
use activitystreams::collection::properties::{CollectionPageProperties, CollectionProperties};
use activitystreams::collection::{OrderedCollection, OrderedCollectionPage};
use activitystreams::object::properties::ObjectProperties;
use activitystreams::BaseBox;
use actix_web::{get, web, Either, HttpRequest};
use serde::Deserialize;
use sqlx::Row;
use std::convert::TryFrom;
use tracing::instrument;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize)]
pub struct GetInboxQuery {
	page: Option<bool>,
	#[serde(rename = "max_id")]
	max_count: Option<i64>,
	#[serde(rename = "min_id")]
	min_count: Option<i64>,
}

#[get("/users/{username}/inbox")]
#[instrument]
pub async fn get_inbox(
	state: web::Data<AppState>,
	req: HttpRequest,
	query: web::Query<GetInboxQuery>,
	path: web::Path<String>,
) -> Result<Either<web::Json<OrderedCollection>, web::Json<OrderedCollectionPage>>, ApiError> {
	match account::ensure_signed_in(&state, &req) {
		Some(username) if username == *path => (),
		Some(_) => return Err(ApiError::Forbidden),
		None => return Err(ApiError::NotSignedIn),
	}

	if let Some(page) = query.page {
		if page {
			return get_inbox_page(state, query, path).await.map(Either::Right);
		}
	}

	get_inbox_index(state, path).await.map(Either::Left)
}

#[instrument]
async fn get_inbox_index(
	state: web::Data<AppState>,
	path: web::Path<String>,
) -> Result<web::Json<OrderedCollection>, ApiError> {
	let username = path.into_inner();
	let user_exists: bool = sqlx::query(
		"SELECT EXISTS(SELECT 1 FROM users WHERE username = $1 AND this_instance = TRUE)",
	)
	.bind(&username)
	.fetch_one(&state.db)
	.await?
	.get(0);
	if !user_exists {
		return Err(ApiError::UserDoesNotExist);
	}

	let user_id: Uuid =
		sqlx::query("SELECT id FROM users WHERE username = $1 AND this_instance = TRUE")
			.bind(&username)
			.fetch_one(&state.db)
			.await?
			.get(0);

	let mut collection = OrderedCollection::new();
	let collection_props: &mut ObjectProperties = collection.as_mut();

	collection_props.set_context_xsd_any_uri("https://www.w3.org/ns/activitystreams")?;
	collection_props.set_id(format!("{}/inbox", url::activitypub_actor(&username)))?;

	let collection_props: &mut CollectionProperties = collection.as_mut();

	let total_items: i64 = sqlx::query(
		"SELECT COUNT(1) FROM activities WHERE $1 = ANY(to_mentions) OR $1 = ANY(cc_mentions)",
	)
	.bind(user_id)
	.fetch_one(&state.db)
	.await?
	.get(0);
	collection_props
		.set_total_items(u64::try_from(total_items).map_err(|_| ApiError::InternalServerError)?)?;
	collection_props.set_first_xsd_any_uri(format!(
		"{}://{}/users/{}/inbox?page=true",
		state.scheme, state.domain, username
	))?;
	collection_props.set_last_xsd_any_uri(format!(
		"{}://{}/users/{}/inbox?min_id=0&page=true",
		state.scheme, state.domain, username
	))?;

	Ok(web::Json(collection))
}

#[instrument]
async fn get_inbox_page(
	state: web::Data<AppState>,
	query: web::Query<GetInboxQuery>,
	path: web::Path<String>,
) -> Result<web::Json<OrderedCollectionPage>, ApiError> {
	if query.max_count.is_some() && query.min_count.is_some() {
		todo!("specifying both max_count and min_count");
	}

	if query.max_count.is_some() {
		return get_inbox_max_count(state, query, path).await;
	} else if query.min_count.is_some() {
		return get_inbox_min_count(state, query, path).await;
	}

	get_inbox_first_page(state, path).await
}

#[instrument]
async fn get_inbox_first_page(
	state: web::Data<AppState>,
	path: web::Path<String>,
) -> Result<web::Json<OrderedCollectionPage>, ApiError> {
	let username = path.into_inner();
	let user_exists: bool = sqlx::query(
		"SELECT EXISTS(SELECT 1 FROM users WHERE username = $1 AND this_instance = TRUE)",
	)
	.bind(&username)
	.fetch_one(&state.db)
	.await?
	.get(0);
	if !user_exists {
		return Err(ApiError::UserDoesNotExist);
	}

	let user_id: Uuid =
		sqlx::query("SELECT id FROM users WHERE username = $1 AND this_instance = TRUE")
			.bind(&username)
			.fetch_one(&state.db)
			.await?
			.get(0);

	let mut page = OrderedCollectionPage::new();
	let page_props: &mut ObjectProperties = page.as_mut();

	page_props.set_context_xsd_any_uri("https://www.w3.org/ns/activitystreams")?;

	let part_of_id_url = format!("{}/inbox", url::activitypub_actor(&username));
	page_props.set_id(format!("{}?page=true", part_of_id_url))?;

	let page_props: &mut CollectionPageProperties = page.as_mut();

	page_props.set_part_of_xsd_any_uri(part_of_id_url.as_str())?;

	let rows = sqlx::query("SELECT count, activity FROM activities WHERE $1 = ANY(to_mentions) OR $1 = ANY(cc_mentions) ORDER BY count DESC LIMIT 20")
        .bind(user_id)
        .fetch_all(&state.db)
        .await?;

	if !rows.is_empty() {
		let first_count: i64 = rows[0].get(0);
		let last_count: i64 = rows[rows.len() - 1].get(0);
		let activities: Result<Vec<BaseBox>, serde_json::Error> = rows
			.iter()
			.map(|row| serde_json::from_value(row.get(1)))
			.collect();
		let activities = activities.map_err(|_| ApiError::InternalServerError)?;

		page_props.set_prev_xsd_any_uri(format!(
			"{}?min_id={}&page=true",
			part_of_id_url, first_count
		))?;
		if rows.len() == 20 {
			page_props.set_next_xsd_any_uri(format!(
				"{}?max_id={}&page=true",
				part_of_id_url, last_count
			))?;
		}

		let page_props: &mut CollectionProperties = page.as_mut();
		page_props.set_many_items_base_boxes(activities)?;
	}

	Ok(web::Json(page))
}

#[instrument]
async fn get_inbox_max_count(
	state: web::Data<AppState>,
	query: web::Query<GetInboxQuery>,
	path: web::Path<String>,
) -> Result<web::Json<OrderedCollectionPage>, ApiError> {
	let username = path.into_inner();
	let user_exists: bool = sqlx::query(
		"SELECT EXISTS(SELECT 1 FROM users WHERE username = $1 AND this_instance = TRUE)",
	)
	.bind(&username)
	.fetch_one(&state.db)
	.await?
	.get(0);
	if !user_exists {
		return Err(ApiError::UserDoesNotExist);
	}

	let user_id: Uuid =
		sqlx::query("SELECT id FROM users WHERE username = $1 AND this_instance = TRUE")
			.bind(&username)
			.fetch_one(&state.db)
			.await?
			.get(0);

	let max_count = query.max_count.unwrap();

	let mut page = OrderedCollectionPage::new();
	let page_props: &mut ObjectProperties = page.as_mut();

	page_props.set_context_xsd_any_uri("https://www.w3.org/ns/activitystreams")?;

	let part_of_id_url = format!("{}/inbox", url::activitypub_actor(&username));

	page_props.set_id(format!("{}?max_id={}&page=true", part_of_id_url, max_count))?;

	let page_props: &mut CollectionPageProperties = page.as_mut();

	page_props.set_part_of_xsd_any_uri(part_of_id_url.as_str())?;

	let rows = sqlx::query("SELECT count, activity FROM activities WHERE ($1 = ANY(to_mentions) OR $1 = ANY(cc_mentions)) AND count < $2 ORDER BY count DESC LIMIT 20")
        .bind(user_id)
        .bind(max_count)
        .fetch_all(&state.db)
        .await?;

	if !rows.is_empty() {
		let first_count: i64 = rows[0].get(0);
		let last_count: i64 = rows[rows.len() - 1].get(0);
		let activities: Result<Vec<BaseBox>, serde_json::Error> = rows
			.iter()
			.map(|row| serde_json::from_value(row.get(1)))
			.collect();
		let activities = activities.map_err(|_| ApiError::InternalServerError)?;

		page_props.set_prev_xsd_any_uri(format!(
			"{}?min_id={}&page=true",
			part_of_id_url, first_count
		))?;
		if rows.len() == 20 {
			page_props.set_next_xsd_any_uri(format!(
				"{}?max_id={}&page=true",
				part_of_id_url, last_count
			))?;
		}

		let page_props: &mut CollectionProperties = page.as_mut();
		page_props.set_many_items_base_boxes(activities)?;
	}

	Ok(web::Json(page))
}

#[instrument]
async fn get_inbox_min_count(
	state: web::Data<AppState>,
	query: web::Query<GetInboxQuery>,
	path: web::Path<String>,
) -> Result<web::Json<OrderedCollectionPage>, ApiError> {
	let username = path.into_inner();
	let user_exists: bool = sqlx::query(
		"SELECT EXISTS(SELECT 1 FROM users WHERE username = $1 AND this_instance = TRUE)",
	)
	.bind(&username)
	.fetch_one(&state.db)
	.await?
	.get(0);
	if !user_exists {
		return Err(ApiError::UserDoesNotExist);
	}

	let user_id: Uuid =
		sqlx::query("SELECT id FROM users WHERE username = $1 AND this_instance = TRUE")
			.bind(&username)
			.fetch_one(&state.db)
			.await?
			.get(0);

	let min_count = query.min_count.unwrap();

	let mut page = OrderedCollectionPage::new();
	let page_props: &mut ObjectProperties = page.as_mut();

	page_props.set_context_xsd_any_uri("https://www.w3.org/ns/activitystreams")?;

	let part_of_id_url = format!("{}/inbox", url::activitypub_actor(&username));

	page_props.set_id(format!("{}?min_id={}&page=true", part_of_id_url, min_count))?;

	let page_props: &mut CollectionPageProperties = page.as_mut();

	page_props.set_part_of_xsd_any_uri(part_of_id_url.as_str())?;

	let rows = sqlx::query("SELECT * FROM (SELECT count, activity FROM activities WHERE ($1 = ANY(to_mentions) OR $1 = ANY(cc_mentions)) AND count > $2 ORDER BY count LIMIT 20) AS tmp ORDER BY count DESC")
        .bind(user_id)
        .bind(min_count)
        .fetch_all(&state.db)
        .await?;

	if !rows.is_empty() {
		let first_count: i64 = rows[0].get(0);
		let last_count: i64 = rows[rows.len() - 1].get(0);
		let activities: Result<Vec<BaseBox>, serde_json::Error> = rows
			.iter()
			.map(|row| serde_json::from_value(row.get(1)))
			.collect();
		let activities = activities.map_err(|_| ApiError::InternalServerError)?;

		page_props.set_prev_xsd_any_uri(format!(
			"{}?min_id={}&page=true",
			part_of_id_url, first_count
		))?;
		if rows.len() == 20 {
			page_props.set_next_xsd_any_uri(format!(
				"{}?max_id={}&page=true",
				part_of_id_url, last_count
			))?;
		}

		let page_props: &mut CollectionProperties = page.as_mut();
		page_props.set_many_items_base_boxes(activities)?;
	}

	Ok(web::Json(page))
}
