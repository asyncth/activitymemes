CREATE TABLE users (
	id uuid PRIMARY KEY,
	username text NOT NULL,
	this_instance boolean NOT NULL,
	instance_url text,
	email text,
	password text,
	name text,
	bio text,
	profile_picture_id text
);

CREATE INDEX users_username_idx ON users (username);
CREATE INDEX users_instance_url_idx ON users (instance_url);

CREATE TABLE activities (
	id uuid PRIMARY KEY,
	user_id uuid REFERENCES users (id) NOT NULL,
	this_instance boolean NOT NULL,
	published_at timestamp WITHOUT TIME ZONE DEFAULT (NOW() AT TIME ZONE 'utc') NOT NULL,
	activity jsonb NOT NULL,
	is_public boolean NOT NULL,
	to_mentions uuid[] NOT NULL,
	cc_mentions uuid[] NOT NULL,
	to_followers_of uuid[] NOT NULL,
	cc_followers_of uuid[] NOT NULL
);

CREATE TABLE follows (
	subject_user_id uuid REFERENCES users (id) NOT NULL,
	object_user_id uuid REFERENCES users (id) NOT NULL,
	following_since timestamp WITHOUT TIME ZONE DEFAULT (NOW() AT TIME ZONE 'utc') NOT NULL,
	pending boolean NOT NULL,
	PRIMARY KEY (subject_user_id, object_user_id)
);

CREATE INDEX follows_pending_idx ON follows (pending);
