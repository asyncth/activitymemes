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

#![forbid(unsafe_code)]

mod account;
mod activitypub;
mod config;
mod endpoints;
mod error;
mod routines;
mod state;
mod url;
mod web_finger;

use actix_web::{web, App, HttpServer};
use config::Config;
use sqlx::migrate::Migrator;
use state::AppState;
use std::error::Error;
use std::process;
use tracing::{instrument, Level};
use tracing_subscriber::FmtSubscriber;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

#[actix_web::main]
async fn main() {
	if let Err(e) = run().await {
		eprintln!("Fatal: {}", e);
		process::exit(1);
	}
}

#[instrument]
async fn run() -> Result<(), Box<dyn Error>> {
	let subscriber = FmtSubscriber::builder()
		.with_max_level(Level::INFO)
		.finish();

	tracing::subscriber::set_global_default(subscriber)
		.expect("expected tracing::subscriber::set_global_default to succeed");

	let config = Config::with_file("config.json")?;
	let state = web::Data::new(AppState::new(config).await?);

	// Run database migrations.
	MIGRATOR.run(&state.db).await?;

	url::init(&state);

	HttpServer::new(move || {
		App::new()
			.app_data(state.clone())
			.service(web_finger::web_finger)
			.service(activitypub::outbox::post_to_outbox)
			.service(activitypub::activities::get_activity)
			.service(
				web::scope("/users")
					.service(endpoints::users::get_user)
					.service(endpoints::users::get_inbox)
					.service(endpoints::users::get_outbox)
					.service(endpoints::users::get_followers)
					.service(endpoints::users::get_following),
			)
			.service(
				web::scope("/account")
					.service(account::sign_up::sign_up)
					.service(account::sign_in::sign_in)
					.service(account::sign_out::sign_out),
			)
	})
	.bind("127.0.0.1:8080")?
	.run()
	.await?;

	Ok(())
}
