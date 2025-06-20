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

### Required Environment Variables

```bash
# Zalo Bot Configuration
export ZALO_BOT_TOKEN=your_bot_token_here

# GitHub Configuration (for image hosting)
export GITHUB_TOKEN=your_github_token_here # Needs 'repo' scope
export GITHUB_REPOSITORY=your_repository_name # Auto-set in GitHub Actions

# Optional: Specific release ID (or use --use-latest-release)
export GITHUB_RELEASE_ID=123456
```

### GitHub Setup

1. **Create a GitHub repository** for storing question images (e.g., `gmat-bot-images`)

2. **Generate a Personal Access Token:**
   - Go to GitHub Settings > Developer settings > Personal access tokens
   - Create token with `repo` scope
   - Copy the token value

3. **Create a GitHub release:**
   ```bash
   # Option 1: Let the bot create one
   cargo run -- --bot-service --create-release --release-tag v1.0.0

   # Option 2: Create manually on GitHub web interface
   # Go to your repo > Releases > Create a new release
   ```

### Zalo Bot Setup

1. **Create a Zalo Bot:**
   - Visit Zalo Developer Portal
   - Create a new bot application
   - Get your bot token

2. **Test the bot:**
   - Add your bot to a Zalo chat
   - Send a message to get the bot token working

## Usage

### 1. Bot Service Mode (Recommended)

Start the bot service that continuously listens for messages and responds to user requests:

```bash
# Start service using latest GitHub release
cargo run -- --bot-service --use-latest-release

# Start service creating a new release
cargo run -- --bot-service --create-release --release-tag v1.0.0

# Start service with specific release ID
cargo run -- --bot-service --github-release-id 123456
```

The bot will:
- Use 24-hour long polling to wait for user messages
- Parse user messages for question type requests (RC, SC, CR, PS, DS)
- Respond with appropriate GMAT question images or help messages
- Upload images to GitHub releases for hosting

**User Interaction:**
- Users send: `"PS"` or `"ps"` → Bot sends a Problem Solving question
- Users send: `"DS"` → Bot sends a Data Sufficiency question
- Users send: `"hello"` → Bot sends help message with available types

### 2. One-time Send to Recent Chats

Generate questions and send them to users who have recently messaged your bot:

```bash
# Generate and send 1 question to recent chats
cargo run -- --question-type sc --send-zalo --use-latest-release

# Generate and send 3 questions to recent chats
cargo run -- --question-type ps --count 3 --generate-images --send-zalo --use-latest-release
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

### 5. GitHub Actions (Automated Daily Execution)

The bot includes a GitHub Actions workflow that runs daily:

1. **Setup Repository Secrets:**
   - `ZALO_BOT_TOKEN` - Your Zalo bot token
   - `GITHUB_TOKEN` - GitHub personal access token with `repo` scope
   - `GITHUB_RELEASE_ID` - (Optional) Specific release ID

   **Note:** `GITHUB_REPOSITORY` is automatically provided by GitHub Actions.

2. **Workflow Features:**
   - Runs daily at 8:00 AM UTC
   - 24-hour continuous polling
   - Automatic dependency caching
   - Error handling and cleanup
   - Manual trigger support

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
| `--github-repo` | GitHub repository name | From `GITHUB_REPOSITORY` env |
| `--github-release-id` | GitHub release ID | From `GITHUB_RELEASE_ID` env |
| `--github-token` | GitHub token | From `GITHUB_TOKEN` env |
| `--create-release` | Create a new GitHub release | - |
| `--use-latest-release` | Use latest GitHub release | - |
| `--release-tag` | Tag name for new releases | "v1.0.0" |

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

The bot integrates with multiple APIs:

### Zalo Bot API
- **getUpdates**: 24-hour long polling to receive user messages
- **sendPhoto**: Send question images using GitHub-hosted URLs
- **sendMessage**: Send text responses and help messages

### GitHub API
- **Releases**: Get release information and upload URLs
- **Assets**: Upload question images as release assets
- **Release Management**: Create and manage releases programmatically

### GMAT Database API
- **Question Index**: Fetch available question IDs by type
- **Question Content**: Retrieve full question data and metadata

## Message Flow

1. **User sends message** → Bot receives via `getUpdates`
2. **Message parsing** → Check if message matches question type (RC/SC/CR/PS/DS)
3. **Valid type** → Generate question image → Upload to GitHub → Send photo
4. **Invalid type** → Send helpful text message with available options
5. **24-hour cycle** → Continuous polling with daily GitHub Actions execution

## GitHub Actions Setup

### Repository Secrets

Add these secrets to your GitHub repository (Settings > Secrets and variables > Actions):

```
ZALO_BOT_TOKEN=your_zalo_bot_token
GITHUB_TOKEN=your_github_personal_access_token
GITHUB_REPOSITORY=your_image_repository_name
GITHUB_RELEASE_ID=your_release_id  # Optional
```

**Note:** `GITHUB_REPOSITORY` is automatically provided by GitHub Actions and doesn't need to be set as a secret.

### Manual Workflow Trigger

You can manually start the bot using GitHub Actions:

1. Go to your repository on GitHub
2. Click "Actions" tab
3. Select "GMAT Zalo Bot Daily Runner"
4. Click "Run workflow"

The workflow will:
- Install dependencies and build the bot
- Run for 24 hours responding to user messages
- Clean up resources automatically
- Provide execution summary

## Troubleshooting

### Common Issues

1. **"Release not found" error:**
   ```bash
   # Create a release first
   cargo run -- --create-release --release-tag v1.0.0
   ```

2. **GitHub upload failed:**
   - Check token has `repo` scope
   - Verify repository exists and token has write access
   - Ensure release exists

3. **Zalo API errors:**
   - Verify bot token is correct
   - Check bot is added to chat/group
   - Ensure users have sent recent messages

### Debug Mode

Run with debug logging:
```bash
RUST_LOG=debug cargo run -- --bot-service --use-latest-release
```
