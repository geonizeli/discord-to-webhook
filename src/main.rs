use std::{env, time::Duration};

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::task::JoinSet;
use tracing::{error, info, warn};
use twilight_gateway::{CloseFrame, Config, Event, EventTypeFlags, Intents, Shard, StreamExt as _};
use twilight_http::Client as DiscordHttpClient;
use twilight_model::{
    channel::message::MessageType,
    gateway::payload::incoming::MessageCreate,
    id::{marker::ChannelMarker, Id},
};
use url::Url;

#[derive(Debug, Clone)]
struct Settings {
    discord_token: String,
    webhook_url: Url,
    channel_id: Id<ChannelMarker>,
}

#[derive(Debug, Serialize)]
struct WebhookPayload<'a> {
    id: String,
    channel_id: String,
    guild_id: Option<String>,
    author_id: String,
    author_name: &'a str,
    author_discriminator: Option<String>,
    author_bot: bool,
    content: &'a str,
    timestamp: String,
    attachments: Vec<AttachmentPayload<'a>>,
    embeds: usize,
    message_url: Option<String>,
}

#[derive(Debug, Serialize)]
struct AttachmentPayload<'a> {
    id: String,
    filename: &'a str,
    url: &'a str,
    proxy_url: &'a str,
    content_type: Option<&'a str>,
    size: u64,
}

impl Settings {
    fn from_env() -> Result<Self> {
        let discord_token = require_env("DISCORD_TOKEN")?;
        let webhook_url = require_env("N8N_WEBHOOK_URL")?
            .parse::<Url>()
            .context("N8N_WEBHOOK_URL must be a valid URL")?;
        let channel_id = require_env("CHANNEL_ID")?
            .parse::<u64>()
            .context("CHANNEL_ID must be a Discord numeric channel id")?;

        Ok(Self {
            discord_token,
            webhook_url,
            channel_id: Id::new(channel_id),
        })
    }
}

fn require_env(name: &str) -> Result<String> {
    let value =
        env::var(name).with_context(|| format!("{name} environment variable is required"))?;
    if value.trim().is_empty() {
        return Err(anyhow!("{name} environment variable cannot be empty"));
    }
    Ok(value)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            env::var("RUST_LOG")
                .unwrap_or_else(|_| "discord_to_webhook=info,twilight_gateway=info".into()),
        )
        .init();

    let settings = Settings::from_env()?;
    let http = DiscordHttpClient::new(settings.discord_token.clone());
    let config = Config::new(
        settings.discord_token.clone(),
        Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT,
    );
    let shards = twilight_gateway::create_recommended(&http, config, |_, builder| builder.build())
        .await
        .context("failed to create Discord gateway shards")?;

    let mut senders = Vec::new();
    let mut tasks = JoinSet::new();
    let shard_count = shards.len();

    for shard in shards {
        senders.push(shard.sender());
        tasks.spawn(run_shard(shard, settings.clone()));
    }

    info!(
        channel_id = %settings.channel_id,
        shards = shard_count,
        "discord-to-webhook bridge started"
    );
    info!("Discord Message Content intent must also be enabled in the Developer Portal for message bodies, attachments, and embeds");

    tokio::signal::ctrl_c()
        .await
        .context("failed to listen for shutdown signal")?;
    SHUTDOWN.store(true, Ordering::Relaxed);

    for sender in senders {
        let _ = sender.close(CloseFrame::NORMAL);
    }

    while let Some(result) = tasks.join_next().await {
        if let Err(error) = result {
            error!(%error, "shard task failed");
        }
    }

    info!("discord-to-webhook bridge stopped");
    Ok(())
}

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

async fn run_shard(mut shard: Shard, settings: Settings) {
    while let Some(item) = shard.next_event(EventTypeFlags::all()).await {
        let event = match item {
            Ok(Event::GatewayClose(_)) if SHUTDOWN.load(Ordering::Relaxed) => break,
            Ok(event) => event,
            Err(error) => {
                warn!(%error, shard = ?shard.id(), "error receiving Discord gateway event");
                continue;
            }
        };

        if let Err(error) = handle_event(&settings, event).await {
            error!(%error, shard = ?shard.id(), "failed to handle Discord event");
        }
    }
}

async fn handle_event(settings: &Settings, event: Event) -> Result<()> {
    match event {
        Event::Ready(ready) => info!(user = %ready.user.name, "connected to Discord gateway"),
        Event::MessageCreate(message) => forward_message(settings, &message).await?,
        Event::GatewayClose(close) => warn!(?close, "Discord gateway closed"),
        Event::GatewayReconnect => warn!("Discord requested gateway reconnect"),
        _ => {}
    }
    Ok(())
}

async fn forward_message(settings: &Settings, message: &MessageCreate) -> Result<()> {
    if message.channel_id != settings.channel_id
        || message.author.bot
        || message.kind != MessageType::Regular
    {
        return Ok(());
    }

    if message.content.is_empty() && message.attachments.is_empty() && message.embeds.is_empty() {
        warn!(
            message_id = %message.id,
            "message had no content, attachments, or embeds; verify the Discord Message Content privileged intent is enabled"
        );
    }

    let payload = WebhookPayload {
        id: message.id.to_string(),
        channel_id: message.channel_id.to_string(),
        guild_id: message.guild_id.map(|id| id.to_string()),
        author_id: message.author.id.to_string(),
        author_name: &message.author.name,
        author_discriminator: Some(message.author.discriminator.to_string()),
        author_bot: message.author.bot,
        content: &message.content,
        timestamp: message.timestamp.iso_8601().to_string(),
        attachments: message
            .attachments
            .iter()
            .map(|attachment| AttachmentPayload {
                id: attachment.id.to_string(),
                filename: &attachment.filename,
                url: &attachment.url,
                proxy_url: &attachment.proxy_url,
                content_type: attachment.content_type.as_deref(),
                size: attachment.size,
            })
            .collect(),
        embeds: message.embeds.len(),
        message_url: message.guild_id.map(|guild_id| {
            format!(
                "https://discord.com/channels/{}/{}/{}",
                guild_id, message.channel_id, message.id
            )
        }),
    };

    Client::new()
        .post(settings.webhook_url.clone())
        .json(&payload)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .context("failed to send webhook request")?
        .error_for_status()
        .context("webhook endpoint returned an error status")?;

    info!(message_id = %message.id, "forwarded Discord message to webhook");
    Ok(())
}
