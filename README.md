# moyn-cli

Developer microblogging from your terminal. Publish markdown posts to [Moyn](https://moyn.dev) without leaving your workflow.

## Installation

### From source

```bash
git clone https://github.com/moyn-dev/moyn-cli.git
cd moyn-cli
cargo build --release
# Binary is at ./target/release/moyn
```

### With Cargo

```bash
cargo install --git https://github.com/moyn-dev/moyn-cli.git
```

## Setup

1. Get your API token from your Moyn profile page
2. Run `moyn login` and paste your token

```bash
moyn login
# Enter your API token (from your profile page): moyn_xxxxx
# Enter API URL [https://moyn.dev]:
```

## Usage

### Publish a post

```bash
moyn publish post.md
```

Posts are markdown files with optional YAML frontmatter:

```markdown
---
title: My Post Title
published: true
tags: [rust, cli]
slug: custom-url-slug
space: my-space
---

Your content here...
```

If no frontmatter title is provided, the first `# heading` or filename is used.

### List your posts

```bash
moyn posts
```

### Delete a post

```bash
moyn delete <post-id>
```

### Spaces

List your spaces:

```bash
moyn spaces
```

Create a space:

```bash
moyn space create --name "My Space" --visibility private
moyn space create --name "Dev Notes" --slug dev --visibility unlisted
```

Show space details:

```bash
moyn space show <slug>
```

## License

MIT
