use crate::{config, session};
use anyhow::{Error, Result};
use matrix_sdk::{
    config::SyncSettings,
    ruma::{
        api::client::filter::FilterDefinition, events::room::message::RoomMessageEventContent,
        RoomId,
    },
    Client,
};
use tracing::{info, warn};

struct Message {
    app: String,
    title: Option<String>,
    message: String,
}

impl Message {
    pub fn format(&self) -> String {
        format!(
            "### {}: {}\n{}",
            self.app,
            self.title.clone().unwrap_or_default(),
            self.message
        )
    }
}

pub async fn run(config: config::Config) -> Result<()> {
    let data_dir = &config.matrix.session_dir;
    let session_file = data_dir.join("session");

    let (client, last_id) = if session_file.exists() {
        session::restore_session(&session_file).await?
    } else {
        (
            session::login(&config, &data_dir, &session_file).await?,
            None,
        )
    };

    let gotify_client: gotify::ClientClient =
        gotify::Client::new(config.gotify.url.as_str(), &config.gotify.token)?;
    sync(client, config, gotify_client, last_id)
        .await
        .map_err(Into::into)
}

async fn sync_gotify_messages(
    client: Client,
    gotify_client: gotify::ClientClient,
    config: config::Config,
    last_id: Option<i64>,
) -> Result<()> {
    info!("Syncing gotify messages");

    let mut current_id = last_id;
    loop {
        match sync_gotify_messages_loop(&client, &gotify_client, &config, &mut current_id).await {
            Ok(_) => {}
            Err(e) => {
                warn!("Error {:?} in sync_gotify_messages_loop", e);
                warn!("Retrying in 10s...");
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            }
        }
    }
}

async fn sync_gotify_messages_loop(
    client: &Client,
    gotify_client: &gotify::ClientClient,
    config: &config::Config,
    last_id: &mut Option<i64>,
) -> Result<()> {
    let room = client
        .get_room(
            <&RoomId>::try_from(config.matrix.room_id.as_str()).expect("Could not convert room id"),
        )
        .unwrap();
    //
    // get applications
    let apps = gotify_client.get_applications().await?;

    // retrieve all old messages
    let mut msg_builder = gotify_client.get_messages();
    if let Some(id) = last_id {
        msg_builder = msg_builder.with_since(*id);
    }

    let mut paged_msgs = msg_builder.send().await?;
    let mut msgs = paged_msgs.messages;
    while let Some(_) = paged_msgs.paging.next {
        msg_builder = gotify_client
            .get_messages()
            .with_since(paged_msgs.paging.since);
        paged_msgs = msg_builder.send().await?;
        msgs.extend(paged_msgs.messages);
    }

    // send old messages
    for msg in msgs {
        let app = &apps
            .iter()
            .find(|&a| a.id == msg.appid)
            .ok_or(Error::msg("Could not find app from id"))?
            .name;

        let message = Message {
            app: app.to_string(),
            title: msg.title,
            message: msg.message,
        };
        let message = RoomMessageEventContent::text_plain(message.format());
        room.send(message).await.unwrap();
        *last_id = Some(msg.id);
    }
    let session_file = &config.matrix.session_dir.join("session");
    session::persist_last_id(&session_file, *last_id).await?;

    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

    Ok(())
}

/// Setup the client to listen to new messages.
async fn sync(
    client: Client,
    config: config::Config,
    gotify_client: gotify::ClientClient,
    last_id: Option<i64>,
) -> Result<()> {
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

    tokio::spawn(sync_gotify_messages(
        client.clone(),
        gotify_client,
        config,
        last_id,
    ));
    session::sync_loop(client, sync_settings).await?;
    Ok(())
}
