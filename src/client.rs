use crate::{config, session};
use anyhow::{Error, Result};
use futures_util::StreamExt;
use gotify::ClientClient as GotifyClient;
use matrix_sdk::{
    config::SyncSettings,
    ruma::{
        api::client::filter::FilterDefinition, events::room::message::RoomMessageEventContent,
        RoomId,
    },
    Client as MatrixClient, Room,
};
use std::path::Path;
use tracing::{debug, info, warn};

struct Message {
    app: String,
    title: Option<String>,
    message: String,
}

impl Message {
    pub fn format_plain(&self) -> String {
        format!(
            "{} ({}) \n{}",
            self.title.clone().unwrap_or_default(),
            self.app,
            self.message
        )
    }
    pub fn format_html(&self) -> String {
        format!(
            "<h4>{} (<u>{}</u>)</h4>\n{}",
            self.title.clone().unwrap_or_default(),
            self.app,
            self.message
        )
    }
}

struct Converter(Vec<gotify::models::Application>);
impl Converter {
    pub async fn new(client: &GotifyClient) -> Result<Converter> {
        let apps = client.get_applications().await?;
        Ok(Converter(apps))
    }
    pub fn convert(&self, message: &gotify::models::Message) -> Result<RoomMessageEventContent> {
        let app = &self
            .0
            .iter()
            .find(|&a| a.id == message.appid)
            .ok_or(Error::msg("Could not find app from id"))?
            .name;

        let message = Message {
            app: app.to_string(),
            title: message.title.clone(),
            message: message.message.clone(),
        };
        Ok(RoomMessageEventContent::text_html(
            message.format_plain(),
            message.format_html(),
        ))
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

    let gotify_client: GotifyClient =
        gotify::Client::new(config.gotify.url.as_str(), &config.gotify.token)?;
    sync(client, config, gotify_client, last_id)
        .await
        .map_err(Into::into)
}

async fn sync_gotify_messages(
    client: MatrixClient,
    gotify_client: GotifyClient,
    config: config::Config,
    last_id: Option<i64>,
) -> Result<()> {
    info!("Syncing gotify messages...");

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
    client: &MatrixClient,
    gotify_client: &GotifyClient,
    config: &config::Config,
    last_id: &mut Option<i64>,
) -> Result<()> {
    debug!("Syncing gotify messages with last_id: {:?}", last_id);
    let room = client
        .get_room(
            <&RoomId>::try_from(config.matrix.room_id.as_str()).expect("Could not convert room id"),
        )
        .unwrap();
    // get applications
    let converter = Converter::new(gotify_client).await?;

    // retrieve all old messages
    let mut msg_builder = gotify_client.get_messages();
    let mut paged_msgs = msg_builder.send().await?;
    let mut msgs: Vec<_> = paged_msgs
        .messages
        .into_iter()
        .filter(|m| m.id > last_id.unwrap_or(i64::MAX))
        .collect();

    debug!("Got {} gotify messages", msgs.len());

    while paged_msgs.paging.next.is_some() && paged_msgs.paging.since >= last_id.unwrap_or(0) {
        msg_builder = gotify_client
            .get_messages()
            .with_since(paged_msgs.paging.since);
        paged_msgs = msg_builder.send().await?;
        let curr_msgs: Vec<_> = paged_msgs
            .messages
            .into_iter()
            .filter(|m| m.id > last_id.unwrap_or(i64::MAX))
            .collect();
        msgs.extend(curr_msgs);
    }

    msgs.reverse();

    // send old messages
    let session_file = &config.matrix.session_dir.join("session");
    for msg in msgs {
        let message = converter.convert(&msg)?;
        send_and_delete(
            gotify_client,
            message,
            msg.id,
            &room,
            last_id,
            session_file,
            config.gotify.delete_sent,
        )
        .await?;
    }

    // stream messages
    let mut msg_stream = gotify_client.stream_messages().await?;
    while let Some(result) = msg_stream.next().await {
        let msg = result?;
        let message = converter.convert(&msg)?;
        send_and_delete(
            gotify_client,
            message,
            msg.id,
            &room,
            last_id,
            session_file,
            config.gotify.delete_sent,
        )
        .await?;
    }

    Ok(())
}

pub async fn send_and_delete(
    gotify_client: &GotifyClient,
    message: RoomMessageEventContent,
    id: i64,
    room: &Room,
    last_id: &mut Option<i64>,
    session_file: &Path,
    delete: bool,
) -> Result<()> {
    debug!("Send message with id {}", id);
    room.send(message).await.unwrap();
    *last_id = Some(id);
    session::persist_last_id(&session_file, *last_id).await?;

    if delete {
        debug!("Deleting message with id {}", id);
        gotify_client.delete_message(id).await?;
    }

    Ok(())
}

/// Setup the client to listen to new messages.
async fn sync(
    client: MatrixClient,
    config: config::Config,
    gotify_client: GotifyClient,
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
