# discord-to-webhook

A small Rust service that listens for regular messages in one Discord channel and forwards them as JSON to an HTTP webhook, such as an n8n webhook.

## Configuration

Set these environment variables:

| Variable | Description |
| --- | --- |
| `DISCORD_TOKEN` | Discord bot token. |
| `CHANNEL_ID` | Numeric Discord channel ID to watch. |
| `N8N_WEBHOOK_URL` | Destination webhook URL. |
| `RUST_LOG` | Optional log filter; defaults to `discord_to_webhook=info,twilight_gateway=info`. |

## Discord setup

The bridge requests the `GUILD_MESSAGES` and `MESSAGE_CONTENT` gateway intents. Discord requires the Message Content privileged intent to be enabled in the Developer Portal for bots to receive message text, embeds, attachments, components, and polls in most guild messages. If the bridge logs that messages are empty, enable **Message Content Intent** on the bot page for the application.

## Docker Compose

```yaml
services:
  bridge:
    container_name: discord-to-webhook
    image: ghcr.io/YOUR_USER/discord-to-webhook:latest
    restart: unless-stopped
    environment:
      DISCORD_TOKEN: ${DISCORD_TOKEN}
      N8N_WEBHOOK_URL: ${N8N_WEBHOOK_URL}
      CHANNEL_ID: ${CHANNEL_ID}
```

## Webhook payload

The service posts JSON with message metadata, author information, content, attachment metadata, embed count, and a Discord message URL when the message is from a guild.
