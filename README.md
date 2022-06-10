# ActivityMemes
ActivityMemes is an open-source federated meme-sharing platform. Uses
ActivityPub for federation.

# Installation
Warning: ActivityMemes is currently in the early stages of development
and can be considered very unstable. There is no guarantee that every
database schema change will have an appropriate migration.

ActivityMemes is written in the Rust programming language and can be
compiled with a modern version of Rust toolchain.

1. Run `cargo build --release` to build ActivityMemes.
2. Create a PostgreSQL database and a user that will be used to access
the database.
3. Generate an RSA keypair and save the public and the private key into
two PEM files.
4. Create a `config.json` file with following contents:
```json
{
    "scheme": "http",
    "domain": "localhost:8080",
	"port": 8080,
    "db_connection_uri": "postgres://username:password@localhost:5432/db_name",
    "num_of_db_pool_connections": 8,
    "token_rsa_public_key_pem_filepath": "pub.pem",
    "token_rsa_private_key_pem_filepath": "priv.pem"
}
```
5. In `config.json`, replace the value of `db_connection_uri` with your
PostgreSQL connection URI, replace values of
`token_rsa_public_key_pem_filepath` and `token_rsa_private_key_pem_filepath`
with filepaths to public and private key files respectively. Please note
that value of `scheme` field currently should not be changed.
6. Start `./target/release/activitymemes`.

In the future, much of this will be automated.
