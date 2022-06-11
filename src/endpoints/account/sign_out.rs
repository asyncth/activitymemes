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
use actix_web::{post, HttpRequest, HttpResponse};
use tracing::instrument;

#[post("/sign-out")]
#[instrument(skip(req))]
pub async fn post_sign_out(req: HttpRequest) -> Result<HttpResponse, ApiError> {
	if let Some(mut cookie) = req.cookie("session") {
		cookie.make_removal();
		Ok(HttpResponse::Ok().cookie(cookie).finish())
	} else {
		Err(ApiError::NotSignedIn)
	}
}
