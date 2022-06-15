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
use actix_rt::time::Instant;
use actix_web::http::Method;
use actix_web::rt as actix_rt;
use actix_web::web;
use awc::http::header::HttpDate;
use awc::http::{header, StatusCode};
use futures::future;
use rsa::RsaPrivateKey;
use std::collections::HashSet;
use std::str;
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

	let activity = Arc::new(activity);
	let actor_id = Arc::new(actor_id);
	let private_key = Arc::new(private_key);
	let digest = Arc::new(digest);

	recipients.remove(&Url::parse(&actor_id)?);
	let recipients = recipients.into_iter().collect();

	deliver_activity_inner(
		state,
		activity,
		recipients,
		actor_id,
		private_key,
		digest,
		1,
	)
	.await
}

#[instrument(skip(
	state,
	activity,
	recipients,
	actor_id,
	private_key,
	digest,
	attempt_number
))]
async fn deliver_activity_inner(
	state: web::Data<AppState>,
	activity: Arc<serde_json::Value>,
	recipients: Vec<Url>,
	actor_id: Arc<String>,
	private_key: Arc<RsaPrivateKey>,
	digest: Arc<String>,
	attempt_number: u32,
) -> Result<(), ApiError> {
	// Do not retry if it was tried more than 3 times.
	if attempt_number > 3 {
		return Ok(());
	}

	let tasks = recipients
		.into_iter()
		.map(|recipient| {
			actix_rt::spawn(deliver_activity_innermore(
				Arc::clone(&activity),
				recipient,
				Arc::clone(&actor_id),
				Arc::clone(&private_key),
				Arc::clone(&digest),
			))
		})
		.collect::<Vec<_>>();

	let results = future::join_all(tasks).await;
	let failed_delivery_recipients: Vec<Url> = results
		.into_iter()
		.filter_map(|result| match result {
			Ok(inner_result) => Some(inner_result),
			Err(err) => {
				error!(?err, "Failed to join delivery task");
				None
			}
		})
		.filter_map(|result| match result {
			Ok(_) => None,
			Err((recipient, ApiError::FailedDeliveryDueToNetworkError)) => {
				error!("Failed to deliver an activity due to a network error.");
				Some(recipient)
			}
			Err((recipient, err)) => {
				error!(
					?recipient,
					?err,
					"Failed to deliver an activity due to an error."
				);

				None
			}
		})
		.collect();

	if !failed_delivery_recipients.is_empty() {
		let mut queue = state.delivery_retry_queue.lock().unwrap();
		let is_empty = queue.is_empty();

		queue.push_back(FailedDelivery {
			activity,
			recipients: failed_delivery_recipients,
			actor_id,
			private_key,
			digest,
			last_time_tried: Instant::now(),
			tried: attempt_number,
		});

		if is_empty {
			state.delivery_retry_notify.notify_waiters();
		}
	}

	Ok(())
}

#[instrument(skip(state))]
pub async fn retry_deliveries(state: web::Data<AppState>) {
	loop {
		let mut queue = state.delivery_retry_queue.lock().unwrap();
		let first = queue.pop_front();
		drop(queue);

		if let Some(failed_delivery) = first {
			let wait_until = failed_delivery.last_time_tried + Duration::from_secs(3600);
			let now = Instant::now();

			if wait_until > now {
				actix_rt::time::sleep_until(wait_until).await;
			}

			let _ = deliver_activity_inner(
				state.clone(),
				failed_delivery.activity,
				failed_delivery.recipients,
				failed_delivery.actor_id,
				failed_delivery.private_key,
				failed_delivery.digest,
				failed_delivery.tried + 1,
			)
			.await;
		} else {
			state.delivery_retry_notify.notified().await;
		}
	}
}

#[instrument(skip(activity, recipient, actor_id, private_key, digest))]
async fn deliver_activity_innermore(
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
		return Err(ApiError::OtherBadRequest);
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

	let mut response = request
		.await
		.map_err(|_| ApiError::FailedDeliveryDueToNetworkError)?;
	if response.status() == StatusCode::METHOD_NOT_ALLOWED {
		// Is a non-federated server. Don't retry.
		return Err(ApiError::OtherBadRequest);
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
		return Err(ApiError::UnexpectedResponseFromFederatedServer);
	}

	let host_header_val = if let Some(host) = inbox_url.host_str() {
		if let Some(port) = inbox_url.port() {
			format!("{}:{}", host, port)
		} else {
			host.to_string()
		}
	} else {
		return Err(ApiError::UnexpectedResponseFromFederatedServer);
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

	let mut response = request
		.await
		.map_err(|_| ApiError::FailedDeliveryDueToNetworkError)?;
	if response.status().is_success() {
		Ok(())
	} else {
		let body = response.body().await?;
		let body = str::from_utf8(&body);
		let status_code = response.status();

		if let Ok(body) = body {
			error!(
				?status_code,
				?body,
				"Failed to deliver an activity due to non-2xx status code.",
			);
		} else {
			error!(
				?status_code,
				"Failed to deliver an activity due to non-2xx status code.",
			);
		}

		Err(ApiError::UnexpectedResponseFromFederatedServer)
	}
}
