# discord-to-webhook

A small Rust service that listens for regular messages in one Discord channel and forwards them as JSON to any HTTP webhook endpoint.

## Configuration

Set these environment variables:

| Variable | Description |
| --- | --- |
| `DISCORD_TOKEN` | Discord bot token. |
| `CHANNEL_ID` | Numeric Discord channel ID to watch. |
| `WEBHOOK_URL` | Destination webhook URL. This can be an n8n webhook, a custom API route, or any other endpoint that accepts JSON via `POST`. |
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
      WEBHOOK_URL: ${WEBHOOK_URL}
      CHANNEL_ID: ${CHANNEL_ID}
```

## Webhook payload

For each regular, non-bot Discord message in the configured channel, the service sends a `POST` request to `WEBHOOK_URL` with a JSON body like this:

```json
{
  "id": "1234567890123456789",
  "channel_id": "2345678901234567890",
  "guild_id": "3456789012345678901",
  "author_id": "4567890123456789012",
  "author_name": "example-user",
  "author_discriminator": "0",
  "author_bot": false,
  "content": "Hello from Discord!",
  "timestamp": "2026-06-18T12:34:56.789000+00:00",
  "attachments": [
    {
      "id": "5678901234567890123",
      "filename": "image.png",
      "url": "https://cdn.discordapp.com/attachments/.../image.png",
      "proxy_url": "https://media.discordapp.net/attachments/.../image.png",
      "content_type": "image/png",
      "size": 12345
    }
  ],
  "embeds": 1,
  "message_url": "https://discord.com/channels/3456789012345678901/2345678901234567890/1234567890123456789"
}
```

### Payload fields

| Field | Type | Description |
| --- | --- | --- |
| `id` | string | Discord message ID. |
| `channel_id` | string | Discord channel ID where the message was created. |
| `guild_id` | string or `null` | Discord guild/server ID. This is `null` for messages that are not associated with a guild. |
| `author_id` | string | Discord user ID of the message author. |
| `author_name` | string | Username of the message author. |
| `author_discriminator` | string or `null` | Discord discriminator for the author. For migrated Discord usernames this is usually `"0"`. |
| `author_bot` | boolean | Whether the author is a bot. The bridge ignores bot messages, so forwarded messages should normally be `false`. |
| `content` | string | Message text content. This can be empty when a message only has attachments/embeds or when the Message Content intent is not enabled. |
| `timestamp` | string | Discord message timestamp in ISO 8601 format. |
| `attachments` | array | Attachment metadata for files uploaded with the message. The bridge forwards metadata and URLs, not file bytes. |
| `attachments[].id` | string | Discord attachment ID. |
| `attachments[].filename` | string | Original attachment filename. |
| `attachments[].url` | string | Discord CDN URL for the attachment. |
| `attachments[].proxy_url` | string | Discord media proxy URL for the attachment. |
| `attachments[].content_type` | string or `null` | Attachment MIME type when Discord provides one. |
| `attachments[].size` | number | Attachment size in bytes. |
| `embeds` | number | Number of embeds on the message. Embed contents are not forwarded. |
| `message_url` | string or `null` | Browser URL for the Discord message when the message is from a guild. |
