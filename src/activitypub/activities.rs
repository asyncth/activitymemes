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
use crate::{account, AppState};
use actix_web::{get, web, HttpRequest};
use serde_json::Value as JsonValue;
use sqlx::Row;
use tracing::instrument;
use uuid::Uuid;

#[get("/activities/{id}")]
#[instrument]
pub async fn get_activity(
	state: web::Data<AppState>,
	req: HttpRequest,
	path: web::Path<String>,
) -> Result<web::Json<JsonValue>, ApiError> {
	let activity_id = path.into_inner();
	let activity_id = Uuid::parse_str(&activity_id).map_err(|_| ApiError::ResourceNotFound)?;

	let row = sqlx::query("SELECT activity, is_public, to_mentions, cc_mentions FROM activities WHERE id = $1 AND this_instance = TRUE")
		.bind(activity_id)
		.fetch_optional(&state.db)
		.await?;
	if row.is_none() {
		return Err(ApiError::ResourceNotFound);
	}
	let row = row.unwrap();

	let is_public: bool = row.get(1);
	if is_public {
		let activity: JsonValue = row.get(0);
		return Ok(web::Json(activity));
	}

	let to: Vec<Uuid> = row.get(2);
	let cc: Vec<Uuid> = row.get(3);

	if let Some(username) = account::ensure_signed_in(&state, &req) {
		let user_id: Uuid =
			sqlx::query("SELECT id FROM users WHERE username = $1 AND this_instance = TRUE")
				.bind(username)
				.fetch_one(&state.db)
				.await?
				.get(0);

		if to.contains(&user_id) || cc.contains(&user_id) {
			let activity: JsonValue = row.get(0);
			return Ok(web::Json(activity));
		}
	}

	Err(ApiError::Forbidden)
}
