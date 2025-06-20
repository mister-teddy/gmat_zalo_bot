# GMAT Zalo Bot

A Rust-based bot that generates GMAT practice questions as images and sends them via Zalo Bot API. The bot can operate in multiple modes: generate images locally, send to recent chats, or run as a continuous service that responds to each user message.

## Features

- 🎯 **800+ GMAT Questions**: Access to Reading Comprehension, Sentence Correction, Critical Reasoning, Problem Solving, and Data Sufficiency questions
- 🖼️ **Beautiful Images**: Generates clean, readable question images with serif fonts and minimal design
- 🤖 **Zalo Integration**: Send questions via Zalo Bot API using base64 encoding
- 🔄 **Bot Service Mode**: Continuous polling that responds to each user message with a random question
- 📊 **Question Statistics**: View database statistics and question counts by type
- 🎨 **Customizable**: Configure question types, captions, and output directories

## Prerequisites

1. **Rust**: Install from [rustup.rs](https://rustup.rs/)
2. **wkhtmltoimage**: Required for generating PNG images from HTML
   - macOS: `brew install wkhtmltopdf`
   - Ubuntu: `sudo apt-get install wkhtmltopdf`
   - Windows: Download from [wkhtmltopdf.org](https://wkhtmltopdf.org/downloads.html)
3. **Zalo Bot Token**: Create a bot and get your token from Zalo Developer Portal

## Installation

```bash
git clone <repository-url>
cd gmat_zalo_bot
cargo build --release
```

## Configuration

Set your Zalo bot token as an environment variable:

```bash
export ZALO_BOT_TOKEN=your_bot_token_here
```

Or pass it as a command line argument using `--bot-token`.

## Usage

### 1. Bot Service Mode (Recommended)

Start the bot service that continuously listens for messages and responds with GMAT questions:

```bash
# Start service with random question types
cargo run -- --bot-service

# Start service with specific question type
cargo run -- --bot-service --question-type ps

# Start service with custom caption
cargo run -- --bot-service --caption "Daily GMAT practice! 📚"
```

The bot will:
- Use long polling to wait for user messages
- Respond to each message with a random GMAT question image
- Run continuously until stopped with Ctrl+C

### 2. One-time Send to Recent Chats

Generate questions and send them to users who have recently messaged your bot:

```bash
# Generate and send 1 question to recent chats
cargo run -- --question-type sc --send-zalo

# Generate and send 3 questions to recent chats
cargo run -- --question-type ps --count 3 --generate-images --send-zalo
```

### 3. Generate Images Locally

Generate question images without sending them:

```bash
# Generate 1 random question image
cargo run -- --generate-images

# Generate 5 Problem Solving questions
cargo run -- --question-type ps --count 5 --generate-images

# Save to custom directory
cargo run -- --generate-images --output-dir ./my-questions
```

### 4. View Statistics

See database statistics and question counts:

```bash
cargo run -- --show-stats
```

## Command Line Options

| Option | Description | Default |
|--------|-------------|---------|
| `-q, --question-type` | Filter by question type (rc, sc, cr, ps, ds) | Random |
| `-c, --count` | Number of questions to pick | 1 |
| `--show-stats` | Show database statistics | - |
| `--generate-images` | Generate PNG images | - |
| `--output-dir` | Output directory for images | `output` |
| `--send-zalo` | One-time send to recent chats | - |
| `--bot-service` | Start continuous polling service | - |
| `--bot-token` | Zalo bot token | From `ZALO_BOT_TOKEN` env |
| `--caption` | Custom message caption | "Here's your GMAT question! 📚" |

## Question Types

- **RC** - Reading Comprehension (160+ questions) - *Currently unsupported due to different JSON structure*
- **SC** - Sentence Correction (160 questions)
- **CR** - Critical Reasoning (152 questions)
- **PS** - Problem Solving (180 questions)
- **DS** - Data Sufficiency (146 questions)

## Architecture

The project follows Rust best practices with a clean separation of concerns:

- **`src/main.rs`** - Command line interface and application entry point
- **`src/lib.rs`** - Core library with all business logic:
  - GMAT database fetching and question selection
  - HTML generation with clean, serif typography
  - Image rendering using wkhtmltoimage
  - Zalo Bot API integration with base64 image encoding
  - Long polling service for continuous operation

## API Integration

The bot integrates with Zalo Bot API endpoints:

- **getUpdates**: Long polling to receive user messages (
