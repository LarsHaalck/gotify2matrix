use std::path::{Path, PathBuf};

use matrix_sdk::{config::SyncSettings, matrix_auth::MatrixSession, Client, Error, LoopCtrl};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tracing::info;

use crate::config;

/// The data needed to re-build a client.
#[derive(Debug, Serialize, Deserialize)]
struct ClientSession {
    homeserver: String,
    db_path: PathBuf,
    passphrase: String,
}

/// The full session to persist.
#[derive(Debug, Serialize, Deserialize)]
struct FullSession {
    client_session: ClientSession,
    user_session: MatrixSession,

    #[serde(skip_serializing_if = "Option::is_none")]
    last_id: Option<i64>,
}

/// Restore a previous session.
pub async fn restore_session(session_file: &Path) -> anyhow::Result<(Client, Option<i64>)> {
    info!(
        "Previous session found in '{}'",
        session_file.to_string_lossy()
    );

    // The session was serialized as JSON in a file.
    let serialized_session = fs::read_to_string(session_file).await?;
    let FullSession {
        client_session,
        user_session,
        last_id,
    } = serde_json::from_str(&serialized_session)?;

    // Build the client with the previous settings from the session.
    let client = Client::builder()
        .homeserver_url(client_session.homeserver)
        .sqlite_store(client_session.db_path, Some(&client_session.passphrase))
        .build()
        .await?;

    info!("Restoring session for {}...", user_session.meta.user_id);

    // Restore the Matrix user session.
    client.restore_session(user_session).await?;

    Ok((client, last_id))
}

pub async fn persist_last_id(session_file: &Path, last_id: Option<i64>) -> anyhow::Result<()> {
    // nothing to be done for None
    if last_id.is_none() {
        return Ok(());
    }

    let serialized_session = fs::read_to_string(session_file).await?;
    let mut full_session: FullSession = serde_json::from_str(&serialized_session)?;

    full_session.last_id = last_id;
    let serialized_session = serde_json::to_string(&full_session)?;
    fs::write(session_file, serialized_session).await?;

    Ok(())
}

/// Login with a new device.
pub async fn login(
    config: &config::Config,
    data_dir: &Path,
    session_file: &Path,
) -> anyhow::Result<Client> {
    info!("No previous session found, logging in…");

    let (client, client_session) = build_client(config, data_dir).await?;
    let matrix_auth = client.matrix_auth();

    loop {
        let username = &config.matrix.username;
        let password = &config.matrix.password;
        match matrix_auth
            .login_username(&username, &password)
            .initial_device_display_name("gotify2matrix")
            .await
        {
            Ok(_) => {
                info!("Logged in as {username}");
                break;
            }
            Err(error) => {
                info!("Error logging in: {error}");
                info!("Please try again\n");
            }
        }
    }

    // Persist the session to reuse it later.
    // This is not very secure, for simplicity. If the system provides a way of
    // storing secrets securely, it should be used instead.
    // Note that we could also build the user session from the login response.
    let user_session = matrix_auth
        .session()
        .expect("A logged-in client should have a session");
    let serialized_session = serde_json::to_string(&FullSession {
        client_session,
        user_session,
        last_id: None,
    })?;
    fs::write(session_file, serialized_session).await?;

    info!("Session persisted in {}", session_file.to_string_lossy());

    Ok(client)
}

/// Build a new client.
async fn build_client(
    config: &config::Config,
    data_dir: &Path,
) -> anyhow::Result<(Client, ClientSession)> {
    let mut rng = thread_rng();

    // Generating a subfolder for the database is not mandatory, but it is useful if
    // you allow several clients to run at the same time. Each one must have a
    // separate database, which is a different folder with the SQLite store.
    let db_subfolder: String = (&mut rng)
        .sample_iter(Alphanumeric)
        .take(7)
        .map(char::from)
        .collect();
    let db_path = data_dir.join(db_subfolder);

    // Generate a random passphrase.
    let passphrase: String = (&mut rng)
        .sample_iter(Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();

    // We create a loop here so the user can retry if an error happens.
    loop {
        let homeserver = &config.matrix.homeserver;

        info!("\nChecking homeserver…");
        match Client::builder()
            .homeserver_url(&homeserver)
            // We use the SQLite store, which is enabled by default. This is the crucial part to
            // persist the encryption setup.
            // Note that other store backends are available and you can even implement your own.
            .sqlite_store(&db_path, Some(&passphrase))
            .build()
            .await
        {
            Ok(client) => {
                return Ok((
                    client,
                    ClientSession {
                        homeserver: homeserver.to_string(),
                        db_path,
                        passphrase,
                    },
                ))
            }
            Err(error) => match &error {
                matrix_sdk::ClientBuildError::AutoDiscovery(_)
                | matrix_sdk::ClientBuildError::Url(_)
                | matrix_sdk::ClientBuildError::Http(_) => {
                    info!("Error checking the homeserver: {error}");
                    info!("Please try again\n");
                }
                _ => {
                    // Forward other errors, it's unlikely we can retry with a different outcome.
                    return Err(error.into());
                }
            },
        }
    }
}

pub async fn sync_loop(client: Client, sync_settings: SyncSettings) -> Result<(), Error> {
    // This loops until we kill the program or an error happens.
    client
        .sync_with_result_callback(sync_settings, |_| async move { Ok(LoopCtrl::Continue) })
        .await
}
