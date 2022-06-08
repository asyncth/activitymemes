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
mod follow;
mod image;
mod object;
mod utils;

use crate::error::ApiError;
use crate::state::AppState;
use activitystreams::activity::kind::{
	AcceptType, CreateType, DeleteType, FollowType, LikeType, RemoveType, UpdateType,
};
use activitystreams::activity::{Create, Follow};
use activitystreams::object::kind::{ImageType, NoteType};
use activitystreams::object::{Image, ObjectBox};
use actix_web::{web, HttpResponse};
use tracing::instrument;
use uuid::Uuid;

#[instrument]
pub async fn post_to_outbox(
	state: web::Data<AppState>,
	user_id: Uuid,
	username: &str,
	body: web::Json<ObjectBox>,
) -> Result<HttpResponse, ApiError> {
	Ok(match body {
		// Activities
		body if body.is_kind(CreateType) => {
			let body: Create = body.to_owned().into_concrete().unwrap();
			create::post_create(state, body, user_id, username).await?
		}
		body if body.is_kind(AcceptType) => todo!("AcceptType"),
		body if body.is_kind(DeleteType) => todo!("DeleteType"),
		body if body.is_kind(FollowType) => {
			let body: Follow = body.to_owned().into_concrete().unwrap();
			follow::post_follow(state, body, user_id, username).await?
		}
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
