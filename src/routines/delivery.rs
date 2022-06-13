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

use super::CLIENT;
use crate::error::ApiError;
use crate::signatures;
use crate::state::{AppState, FailedDelivery};
use activitystreams::actor::properties::ApActorProperties;
use activitystreams::actor::Person;
use activitystreams::ext::Ext;
use activitystreams::BaseBox;
use actix_web::http::Method;
use actix_web::rt::time::Instant;
use actix_web::web;
use awc::http::header::HttpDate;
use awc::http::{header, StatusCode};
use futures::future;
use rsa::RsaPrivateKey;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tracing::{error, instrument};
use url::Url;

#[instrument(skip(state, activity, recipients, private_key))]
pub async fn deliver_activity(
	state: web::Data<AppState>,
	activity: serde_json::Value,
	mut recipients: HashSet<Url>,
	actor_id: String,
	private_key: RsaPrivateKey,
) -> Result<(), ApiError> {
	let activity_bytes = serde_json::to_vec(&activity)?;
	let digest = signatures::digest(&activity_bytes);

	let actor_id = Arc::new(actor_id);
	let private_key = Arc::new(private_key);
	let activity = Arc::new(activity);
	let digest = Arc::new(digest);

	recipients.remove(&Url::parse(&actor_id)?);

	let mut tasks = Vec::with_capacity(recipients.len());
	for recipient in recipients {
		tasks.push(actix_web::rt::spawn(deliver_activity_inner(
			Arc::clone(&activity),
			recipient,
			Arc::clone(&actor_id),
			Arc::clone(&private_key),
			Arc::clone(&digest),
		)));
	}

	let results = future::join_all(tasks).await;
	let mut failed_delivery_recipients = Vec::new();

	for result in results {
		if let Err((recipient, err)) = result? {
			error!(?err, "Delivery failed");
			failed_delivery_recipients.push(recipient);
		}
	}

	if !failed_delivery_recipients.is_empty() {
		let mut lock = state.delivery_retry_queue.write().unwrap();
		lock.push_back(FailedDelivery {
			activity,
			recipients: failed_delivery_recipients,
			time_to_retry: Instant::now() + Duration::from_secs(3600),
		});
	}

	Ok(())
}

#[instrument(skip(activity, recipient, private_key, digest))]
async fn deliver_activity_inner(
	activity: Arc<serde_json::Value>,
	recipient: Url,
	actor_id: Arc<String>,
	private_key: Arc<RsaPrivateKey>,
	digest: Arc<String>,
) -> Result<(), (Url, ApiError)> {
	deliver_activity_innermost(activity, recipient.clone(), actor_id, private_key, digest)
		.await
		.map_err(|err| (recipient, err))
}

#[instrument(skip(activity, private_key, digest))]
async fn deliver_activity_innermost(
	activity: Arc<serde_json::Value>,
	recipient: Url,
	actor_id: Arc<String>,
	private_key: Arc<RsaPrivateKey>,
	digest: Arc<String>,
) -> Result<(), ApiError> {
	if recipient.scheme() != "https" {
		// Just ignore the delivery request.
		return Ok(());
	}

	let request = CLIENT.with(|client| {
		client
			.get(recipient.as_str())
			.insert_header((
				header::ACCEPT,
				"application/ld+json; profile=\"https://www.w3.org/ns/activitystreams\"",
			))
			.send()
	});

	let mut response = request.await?;
	if response.status() == StatusCode::METHOD_NOT_ALLOWED {
		// Is a non-federated server. Ignore the delivery request.
		return Ok(());
	}

	let body = response.body().await?;
	let actor: BaseBox = serde_json::from_slice(&body)?;

	let inbox_url = match actor.kind() {
		Some("Person") => {
			let actor: Ext<Person, ApActorProperties> = actor
				.into_concrete()
				.map_err(|_| ApiError::UnexpectedResponseFromFederatedServer)?;
			let ap_actor_props = &actor.extension;

			ap_actor_props.get_inbox().as_url().clone()
		}
		Some(_) => todo!("delivering to non-person actors"),
		None => return Err(ApiError::UnexpectedResponseFromFederatedServer),
	};

	if inbox_url.scheme() != "https" {
		// Ignore the delivery request.
		return Ok(());
	}

	let host_header_val = if let Some(host) = inbox_url.host_str() {
		if let Some(port) = inbox_url.port() {
			format!("{}:{}", host, port)
		} else {
			host.to_string()
		}
	} else {
		// Ignore the delivery request.
		return Ok(());
	};

	let now = SystemTime::now();
	let signature = signatures::sign(
		&format!("{}#main-key", &actor_id),
		Method::POST,
		inbox_url.path(),
		&host_header_val,
		now,
		&digest,
		&private_key,
	)?;

	let request = CLIENT.with(|client| {
		client
			.post(inbox_url.as_str())
			.insert_header((header::HOST, host_header_val))
			.insert_header((header::DATE, HttpDate::from(now)))
			.insert_header(("Digest", (&*digest).clone()))
			.insert_header(("Signature", signature))
			.insert_header((
				header::CONTENT_TYPE,
				"application/ld+json; profile=\"https://www.w3.org/ns/activitystreams\"",
			))
			.send_json(&activity)
	});

	let response = request.await?;
	if response.status().is_success() {
		Ok(())
	} else {
		error!(
			"Failed to deliver an activity. Status code: {}.",
			response.status()
		);
		Err(ApiError::UnexpectedResponseFromFederatedServer)
	}
}
