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
use activitystreams::BaseBox;
use actix_web::{get, web, Either};
use chrono::NaiveDateTime;
use serde::Deserialize;
use sqlx::Row;
use std::convert::TryFrom;
use tracing::instrument;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize)]
pub struct GetOutboxQuery {
	page: Option<bool>,
	#[serde(rename = "max_id")]
	max_timestamp: Option<i64>,
	#[serde(rename = "min_id")]
	min_timestamp: Option<i64>,
}

#[get("/users/{username}/outbox")]
#[instrument]
pub async fn get_outbox(
	state: web::Data<AppState>,
	query: web::Query<GetOutboxQuery>,
	path: web::Path<String>,
) -> Result<Either<web::Json<OrderedCollection>, web::Json<OrderedCollectionPage>>, ApiError> {
	if let Some(page) = query.page {
		if page {
			return get_outbox_page(state, query, path).await.map(Either::Right);
		}
	}

	get_outbox_index(state, path).await.map(Either::Left)
}

#[instrument]
async fn get_outbox_index(
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

	let id_url = format!("{}/outbox", url::activitypub_actor(&username));
	collection_props.set_id(id_url.as_str())?;

	let collection_props: &mut CollectionProperties = collection.as_mut();

	let total_items: i64 = sqlx::query("SELECT COUNT(1) FROM activities WHERE user_id = $1")
		.bind(user_id)
		.fetch_one(&state.db)
		.await?
		.get(0);
	collection_props.set_total_items(u64::try_from(total_items)?)?;
	collection_props.set_first_xsd_any_uri(format!("{}?page=true", id_url))?;
	collection_props.set_last_xsd_any_uri(format!("{}?min_id=0&page=true", id_url))?;

	Ok(web::Json(collection))
}

#[instrument]
async fn get_outbox_page(
	state: web::Data<AppState>,
	query: web::Query<GetOutboxQuery>,
	path: web::Path<String>,
) -> Result<web::Json<OrderedCollectionPage>, ApiError> {
	if query.max_timestamp.is_some() && query.min_timestamp.is_some() {
		todo!("specifying both max_timestamp and min_timestamp");
	}

	if query.max_timestamp.is_some() {
		return get_outbox_max_timestamp(state, query, path).await;
	} else if query.min_timestamp.is_some() {
		return get_outbox_min_timestamp(state, query, path).await;
	}

	get_outbox_first_page(state, path).await
}

#[instrument]
async fn get_outbox_first_page(
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

	let part_of_id_url = format!("{}/outbox", url::activitypub_actor(&username));

	page_props.set_id(format!("{}?page=true", part_of_id_url))?;

	let page_props: &mut CollectionPageProperties = page.as_mut();

	page_props.set_part_of_xsd_any_uri(part_of_id_url.as_str())?;

	let rows = sqlx::query(
		"SELECT published_at, activity FROM activities WHERE user_id = $1 ORDER BY published_at DESC LIMIT 20",
	)
	.bind(user_id)
	.fetch_all(&state.db)
	.await?;

	if !rows.is_empty() {
		let last_timestamp: NaiveDateTime = rows[rows.len() - 1].get(0);

		let activities: Result<Vec<BaseBox>, serde_json::Error> = rows
			.iter()
			.map(|row| serde_json::from_value(row.get(1)))
			.collect();
		let activities = activities?;

		if rows.len() == 20 {
			page_props.set_next_xsd_any_uri(format!(
				"{}?max_id={}&page=true",
				part_of_id_url,
				last_timestamp.timestamp_millis()
			))?;
		}

		let page_props: &mut CollectionProperties = page.as_mut();
		page_props.set_many_items_base_boxes(activities)?;
	}

	Ok(web::Json(page))
}

#[instrument]
async fn get_outbox_max_timestamp(
	state: web::Data<AppState>,
	query: web::Query<GetOutboxQuery>,
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
	let max_timestamp = NaiveDateTime::from_timestamp(max_timestamp / 1000, ((max_timestamp % 1000) * 1000000) as u32);

	let mut page = OrderedCollectionPage::new();
	let page_props: &mut ObjectProperties = page.as_mut();

	page_props.set_context_xsd_any_uri("https://www.w3.org/ns/activitystreams")?;

	let part_of_id_url = format!("{}/outbox", url::activitypub_actor(&username));

	page_props.set_id(format!(
		"{}?max_id={}&page=true",
		part_of_id_url, max_timestamp
	))?;

	let page_props: &mut CollectionPageProperties = page.as_mut();

	page_props.set_part_of_xsd_any_uri(part_of_id_url.as_str())?;

	let rows = sqlx::query("SELECT published_at, activity FROM activities WHERE user_id = $1 AND published_at < $2 ORDER BY published_at DESC LIMIT 20")
        .bind(user_id)
        .bind(max_timestamp)
        .fetch_all(&state.db)
        .await?;

	if !rows.is_empty() {
		let first_timestamp: NaiveDateTime = rows[0].get(0);
		let last_timestamp: NaiveDateTime = rows[rows.len() - 1].get(0);

		let activities: Result<Vec<BaseBox>, serde_json::Error> = rows
			.iter()
			.map(|row| serde_json::from_value(row.get(1)))
			.collect();
		let activities = activities?;

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
		page_props.set_many_items_base_boxes(activities)?;
	}

	Ok(web::Json(page))
}

#[instrument]
async fn get_outbox_min_timestamp(
	state: web::Data<AppState>,
	query: web::Query<GetOutboxQuery>,
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
	let min_timestamp = NaiveDateTime::from_timestamp(min_timestamp / 1000, ((min_timestamp % 1000) * 1000000) as u32);

	let mut page = OrderedCollectionPage::new();
	let page_props: &mut ObjectProperties = page.as_mut();

	page_props.set_context_xsd_any_uri("https://www.w3.org/ns/activitystreams")?;

	let part_of_outbox_url = format!("{}/outbox", url::activitypub_actor(&username));

	page_props.set_id(format!(
		"{}?min_id={}&page=true",
		part_of_outbox_url, min_timestamp
	))?;

	let page_props: &mut CollectionPageProperties = page.as_mut();

	page_props.set_part_of_xsd_any_uri(part_of_outbox_url.as_str())?;

	let rows = sqlx::query("SELECT * FROM (SELECT published_at, activity FROM activities WHERE user_id = $1 AND published_at > $2 ORDER BY published_at LIMIT 20) AS tmp ORDER BY published_at DESC")
        .bind(user_id)
        .bind(min_timestamp)
        .fetch_all(&state.db)
        .await?;

	if !rows.is_empty() {
		let first_timestamp: NaiveDateTime = rows[0].get(0);
		let last_timestamp: NaiveDateTime = rows[rows.len() - 1].get(0);

		let activities: Result<Vec<BaseBox>, serde_json::Error> = rows
			.iter()
			.map(|row| serde_json::from_value(row.get(1)))
			.collect();
		let activities = activities?;

		page_props.set_prev_xsd_any_uri(format!(
			"{}?min_id={}&page=true",
			part_of_outbox_url,
			first_timestamp.timestamp_millis()
		))?;
		if rows.len() == 20 {
			page_props.set_next_xsd_any_uri(format!(
				"{}?max_id={}&page=true",
				part_of_outbox_url,
				last_timestamp.timestamp_millis()
			))?;
		}

		let page_props: &mut CollectionProperties = page.as_mut();
		page_props.set_many_items_base_boxes(activities)?;
	}

	Ok(web::Json(page))
}
