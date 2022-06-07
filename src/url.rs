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
use once_cell::sync::OnceCell;
use regex::Regex;
use uuid::Uuid;

static SHARED_URL: OnceCell<String> = OnceCell::new();
static USER_URL_REGEX: OnceCell<Regex> = OnceCell::new();
static USER_FOLLOWERS_URL_REGEX: OnceCell<Regex> = OnceCell::new();

pub fn init(state: &AppState) {
	SHARED_URL.get_or_init(|| format!("{}://{}", state.scheme, state.domain));
	USER_URL_REGEX.get_or_init(|| {
		Regex::new(&format!(
			"^{}([a-zA-Z0-9_-]+)$",
			regex::escape(&activitypub_actor(""))
		))
		.unwrap()
	});
	USER_FOLLOWERS_URL_REGEX.get_or_init(|| {
		Regex::new(&format!(
			"^{}([a-zA-Z0-9_-]+)/followers$",
			regex::escape(&activitypub_actor(""))
		))
		.unwrap()
	});
}

pub fn shared_url() -> &'static str {
	SHARED_URL
		.get()
		.expect("expected `SHARED_URL` to be initialized")
}

pub fn user_url_regex() -> &'static Regex {
	USER_URL_REGEX
		.get()
		.expect("expected `USER_URL_REGEX` to be initialized")
}

pub fn user_followers_url_regex() -> &'static Regex {
	USER_FOLLOWERS_URL_REGEX
		.get()
		.expect("expected `USER_FOLLOWERS_URL_REGEX` to be initialized")
}

pub fn html_user(username: &str) -> String {
	format!("{}/@{}", shared_url(), username)
}

pub fn activitypub_actor(username: &str) -> String {
	format!("{}/users/{}", shared_url(), username)
}

pub fn activitypub_activity(id: Uuid) -> String {
	format!("{}/activities/{}", shared_url(), id)
}

pub fn activitypub_object(activity_id: Uuid) -> String {
	format!("{}/activities/{}/object", shared_url(), activity_id)
}
