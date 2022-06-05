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

use crate::state::AppState;
use crate::{error::ApiError, url};
use actix_web::{get, web, HttpResponse};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tracing::instrument;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct WebFingerLink {
	rel: String,
	#[serde(rename = "type")]
	kind: String,
	href: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WebFinger {
	subject: Option<String>,
	aliases: Option<Vec<String>>,
	links: Vec<WebFingerLink>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GetWebFingerQuery {
	resource: String,
}

#[get("/.well-known/webfinger")]
#[instrument]
pub async fn get_web_finger(
	state: web::Data<AppState>,
	query: web::Query<GetWebFingerQuery>,
) -> Result<HttpResponse, ApiError> {
	if !query.resource.contains("acct:") || !query.resource.contains('@') {
		return Err(ApiError::IncorrectResourceQuery);
	}

	let web_finger_id = query.resource.replace("acct:", "");
	let web_finger_id = web_finger_id.split('@').collect::<Vec<&str>>();
	if web_finger_id[1] != state.domain {
		return Err(ApiError::UserDoesNotExist);
	}
	let username = web_finger_id[0];

	let user_exists: bool = sqlx::query(
		"SELECT EXISTS(SELECT 1 FROM users WHERE username = $1 AND this_instance = TRUE)",
	)
	.bind(username)
	.fetch_one(&state.db)
	.await?
	.get(0);
	if !user_exists {
		return Err(ApiError::UserDoesNotExist);
	}

	let html_user_page_url = url::html_user(username);
	let relative_actor_url = format!("/users/{}", username);
	let absolute_actor_url = format!("{}://{}{}", state.scheme, state.domain, relative_actor_url);

	Ok(HttpResponse::Ok()
		.content_type("application/jrd+json")
		.append_header((
			"Link",
			format!("<{}>; rel=prefetch; as=fetch", relative_actor_url),
		))
		.json(WebFinger {
			subject: Some(query.into_inner().resource),
			aliases: Some(vec![html_user_page_url.clone(), relative_actor_url]),
			links: vec![
				WebFingerLink {
					rel: String::from("http://webfinger.net/rel/profile-page"),
					kind: String::from("text/html"),
					href: html_user_page_url,
				},
				WebFingerLink {
					rel: String::from("self"),
					kind: String::from("application/activity+json"),
					href: absolute_actor_url,
				},
			],
		}))
}
