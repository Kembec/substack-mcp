# substack-mcp

[![npm](https://img.shields.io/npm/v/@kembec/substack-mcp)](https://www.npmjs.com/package/@kembec/substack-mcp)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![platforms](https://img.shields.io/badge/platforms-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey)](#)

Read and publish to any Substack publication — single Rust binary, no Node.js runtime.

## Prerequisites

- A Substack account
- Your `connect.sid` session cookie — get it in 3 steps:
  1. Log in at [substack.com](https://substack.com)
  2. Open DevTools → **Application** → **Cookies** → `https://substack.com`
  3. Copy the value of the `connect.sid` cookie

  Works with the raw value (`s:abc…`) or the URL-encoded form (`s%3Aabc…`) — the server decodes automatically.

- Your publication URL (e.g. `https://yourname.substack.com`) — only required for draft and publish tools

Credentials are read from environment variables only and are never written to disk.

## Installation

```bash
npm install -g @kembec/substack-mcp
```

Or run directly without installing:

```bash
npx @kembec/substack-mcp
```

## Configuration

Two environment variables:

| Variable | Required for | Example |
|----------|-------------|---------|
| `SUBSTACK_SID` | Authenticated tools | `s:0w29uWWh…` |
| `SUBSTACK_PUBLICATION_URL` | Draft and publish tools | `https://yourname.substack.com` |

### Cursor

Add to `~/.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "substack": {
      "command": "npx",
      "args": ["-y", "@kembec/substack-mcp"],
      "env": {
        "SUBSTACK_SID": "s:your-connect-sid-value",
        "SUBSTACK_PUBLICATION_URL": "https://yourname.substack.com"
      }
    }
  }
}
```

### Claude Desktop

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "substack": {
      "command": "npx",
      "args": ["-y", "@kembec/substack-mcp"],
      "env": {
        "SUBSTACK_SID": "s:your-connect-sid-value",
        "SUBSTACK_PUBLICATION_URL": "https://yourname.substack.com"
      }
    }
  }
}
```

### Codex CLI

Add to `~/.codex/config.toml`:

```toml
[mcp_servers.substack]
command = "npx"
args = ["-y", "@kembec/substack-mcp"]
enabled = true

[mcp_servers.substack.env]
SUBSTACK_SID = "s:your-connect-sid-value"
SUBSTACK_PUBLICATION_URL = "https://yourname.substack.com"
```

## Tools

### Public — no credentials needed

| Tool | Parameters | Description |
|------|-----------|-------------|
| `get_profile` | `slug: string` | Fetch a writer's public profile |
| `get_posts` | `publication_url: string`, `limit?: number`, `offset?: number` | List published posts |
| `get_post` | `publication_url: string`, `post_slug: string` | Get a single post with content |
| `get_comments` | `post_id: number`, `limit?: number` | List comments on a post |
| `get_notes` | `user_id: number`, `limit?: number`, `offset?: number` | List a user's notes |

### Authenticated — requires `SUBSTACK_SID`

| Tool | Parameters | Description |
|------|-----------|-------------|
| `create_note` | `body: string` | Post a note to your profile |
| `like_post` | `post_id: number` | Like a post |

### Publication — requires `SUBSTACK_SID` + `SUBSTACK_PUBLICATION_URL`

| Tool | Parameters | Description |
|------|-----------|-------------|
| `list_drafts` | `limit?: number`, `offset?: number` | List unpublished drafts |
| `create_draft` | `title: string`, `body: string`, `audience?: "everyone"\|"paid"` | Create a new draft |
| `update_draft` | `draft_id: number`, `title?: string`, `body?: string` | Edit an existing draft |
| `publish_post` | `draft_id: number`, `send_email?: boolean` | **Publish a draft** |

> **Warning — `publish_post` is irreversible.** When `send_email` is `true` (default), the post is published **and** emails are sent to all subscribers immediately. Always verify the draft content with `list_drafts` or `get_post` before publishing.

## Building from source

Requires Rust 1.78+:

```bash
git clone https://github.com/Kembec/substack-mcp.git
cd substack-mcp
cargo build --release
./target/release/substack-mcp
```

## License

MIT
