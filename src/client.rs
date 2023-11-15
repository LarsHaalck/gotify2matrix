use crate::{config, session};
use matrix_sdk::{
    config::SyncSettings,
    ruma::{
        api::client::filter::FilterDefinition, events::room::message::RoomMessageEventContent,
        RoomId,
    },
    Client,
};
use tracing::{info, warn};

pub async fn run(config: &config::Config) -> anyhow::Result<()> {
    let data_dir = &config.matrix.session_dir;
    // The file where the session is persisted.
    let session_file = data_dir.join("session");

    let client = if session_file.exists() {
        session::restore_session(&session_file).await?
    } else {
        session::login(config, &data_dir, &session_file).await?
    };

    sync(client, config).await.map_err(Into::into)
}

/// Setup the client to listen to new messages.
async fn sync(client: Client, config: &config::Config) -> anyhow::Result<()> {
    info!("Launching a first sync");

    // Enable room members lazy-loading, it will speed up the initial sync a lot
    // with accounts in lots of rooms.
    // See <https://spec.matrix.org/v1.6/client-server-api/#lazy-loading-room-members>.
    let filter = FilterDefinition::with_lazy_loading();

    let mut sync_settings = SyncSettings::default().filter(filter.into());
    loop {
        match client.sync_once(sync_settings.clone()).await {
            Ok(response) => {
                sync_settings = sync_settings.token(response.next_batch.clone());
                break;
            }
            Err(error) => {
                warn!("An error occurred during initial sync: {error}");
                warn!("Trying again…");
            }
        }
    }

    info!("The client is ready!");
    let room = client
        .get_room(<&RoomId>::try_from(config.matrix.room_id.as_str()).unwrap())
        .unwrap();
    let content = RoomMessageEventContent::text_plain("🎉🎊🥳 let's PARTY!! 🥳🎊🎉");
    room.send(content).await.unwrap();

    session::sync_loop(client, sync_settings).await?;
    Ok(())
}
