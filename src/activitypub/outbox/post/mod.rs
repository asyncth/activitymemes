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

mod create;
mod image;
mod object;

use crate::account;
use crate::error::ApiError;
use crate::state::AppState;
use activitystreams::activity::kind::{
	AcceptType, CreateType, DeleteType, FollowType, LikeType, RemoveType, UpdateType,
};
use activitystreams::activity::Create;
use activitystreams::object::kind::{ImageType, NoteType};
use activitystreams::object::{Image, ObjectBox};
use actix_web::http::header;
use actix_web::{post, web, HttpRequest, HttpResponse};
use sqlx::Row;
use tracing::instrument;
use uuid::Uuid;

#[post("/users/{username}/outbox")]
#[instrument]
pub async fn post_to_outbox(
	state: web::Data<AppState>,
	req: HttpRequest,
	path: web::Path<String>,
	body: web::Json<ObjectBox>,
) -> Result<HttpResponse, ApiError> {
	let content_type = req
		.headers()
		.get(header::CONTENT_TYPE)
		.ok_or(ApiError::OtherBadRequest)?;
	if !(content_type == "application/ld+json; profile=\"https://www.w3.org/ns/activitystreams\""
		|| content_type == "application/activity+json")
	{
		return Err(ApiError::OtherBadRequest);
	}

	match account::ensure_signed_in(&state, &req) {
		Some(username) if username == *path => (),
		Some(_) => return Err(ApiError::Forbidden),
		None => return Err(ApiError::NotSignedIn),
	}

	let username = path.into_inner();
	let user_id: Uuid =
		sqlx::query("SELECT id FROM users WHERE username = $1 AND this_instance = TRUE")
			.bind(&username)
			.fetch_one(&state.db)
			.await?
			.get(0);

	Ok(match body {
		// Activities
		body if body.is_kind(CreateType) => {
			let body: Create = body.to_owned().into_concrete().unwrap();
			create::post_create(state, body, user_id, username).await?
		}
		body if body.is_kind(AcceptType) => todo!("AcceptType"),
		body if body.is_kind(DeleteType) => todo!("DeleteType"),
		body if body.is_kind(FollowType) => todo!("FollowType"),
		body if body.is_kind(LikeType) => todo!("LikeType"),
		body if body.is_kind(RemoveType) => todo!("RemoveType"),
		body if body.is_kind(UpdateType) => todo!("UpdateType"),
		// Non-activity objects
		body if body.is_kind(ImageType) => {
			let body: Image = body.to_owned().into_concrete().unwrap();
			image::post_image(state, body, user_id, username).await?
		}
		body if body.is_kind(NoteType) => todo!("NoteType"),
		// Other
		_ => todo!("Other"),
	})
}
