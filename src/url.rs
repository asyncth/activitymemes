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
use uuid::Uuid;

static SHARED_URL: OnceCell<String> = OnceCell::new();

pub fn init(state: &AppState) {
	SHARED_URL.get_or_init(|| format!("{}://{}", state.scheme, state.domain));
}

fn get() -> &'static String {
	SHARED_URL
		.get()
		.expect("expected `SHARED_URL` to be initialized")
}

pub fn html_user(username: &str) -> String {
	format!("{}/@{}", get(), username)
}

pub fn activitypub_actor(username: &str) -> String {
	format!("{}/users/{}", get(), username)
}

pub fn activitypub_activity(id: Uuid) -> String {
	format!("{}/activities/{}", get(), id)
}

pub fn activitypub_object(activity_id: Uuid) -> String {
	format!("{}/activities/{}/object", get(), activity_id)
}
