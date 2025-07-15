use clap::Parser;
use gmat_zalo_bot::*;
use std::env;
use std::time::Duration;

/// Helper function to create GitHub configuration from command line arguments
async fn setup_github_config(args: &Args) -> Result<GitHubConfig, Box<dyn std::error::Error>> {
    let github_repo = args
        .github_repo
        .clone()
        .or_else(|| env::var("GITHUB_REPOSITORY").ok())
        .unwrap_or_else(|| "gmat-bot-images".to_string());

    let github_token = args
        .github_token
        .clone()
        .or_else(|| env::var("GITHUB_TOKEN").ok())
        .ok_or(
            "GitHub token is required. Set GITHUB_TOKEN environment variable or use --github-token",
        )?;

    let release_id = if args.create_release {
        println!("🏷️  Creating new GitHub release...");
        create_github_release(&github_repo, &github_token, &args.release_tag).await?
    } else if args.use_latest_release {
        println!("🔍 Getting latest release...");
        get_latest_release_id(&github_repo, &github_token).await?
    } else {
        args.github_release_id
            .or_else(|| env::var("GITHUB_RELEASE_ID").ok().and_then(|s| s.parse().ok()))
            .ok_or("GitHub release ID is required. Use --github-release-id, --use-latest-release, or --create-release")?
    };

    Ok(GitHubConfig {
        repo: github_repo,
        release_id,
        token: github_token,
    })
}

/// Send questions to specified users with retry logic
async fn send_question_to_users(
    zalo_bot: &ZaloBot,
    users: &[String],
    question_id: &str,
    question_type: &QuestionType,
    output_dir: &str,
    github_config: &GitHubConfig,
    show_explanations: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    const MAX_RETRIES: usize = 3;
    let mut last_error = None;

    for attempt in 0..MAX_RETRIES {
        match fetch_question_content(question_id).await {
            Ok(content) => {
                let mut all_successful = true;

                for user_id in users {
                    println!("📤 Sending question to user: {}", user_id);
                    if let Err(e) = zalo_bot
                        .send_question(
                            user_id,
                            &content,
                            Some(question_type),
                            output_dir,
                            github_config,
                            show_explanations,
                        )
                        .await
                    {
                        eprintln!("❌ Failed to send to user {}: {}", user_id, e);
                        all_successful = false;
                    } else {
                        println!("✅ Successfully sent to user: {}", user_id);
                    }
                }

                if all_successful {
                    return Ok(());
                }
            }
            Err(e) => {
                last_error = Some(e);
                if attempt < MAX_RETRIES - 1 {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| "Unknown error".into()))
}

#[derive(Parser, Debug)]
#[command(name = "gmat-zalo-bot")]
#[command(
    about = "GMAT Question Bot for Zalo - Pick random questions and send them via Zalo Bot API"
)]
struct Args {
    /// Question type to filter by
    #[arg(short, long, value_enum)]
    question_type: Option<QuestionType>,

    /// Number of questions to pick
    #[arg(short, long, default_value = "1")]
    count: usize,

    /// Show all available question types and counts
    #[arg(long)]
    show_stats: bool,

    /// Output directory for generated images
    #[arg(long, default_value = "output")]
    output_dir: String,

    /// Start bot service with continuous polling (responds to each message)
    #[arg(long)]
    bot_service: bool,

    /// Zalo Bot Token (can also be set via ZALO_BOT_TOKEN environment variable)
    #[arg(long)]
    bot_token: Option<String>,

    /// GitHub repository name (can also be set via GITHUB_REPOSITORY environment variable)
    #[arg(long)]
    github_repo: Option<String>,

    /// GitHub release ID (can also be set via GITHUB_RELEASE_ID environment variable)
    #[arg(long)]
    github_release_id: Option<u64>,

    /// GitHub token (can also be set via GITHUB_TOKEN environment variable)
    #[arg(long)]
    github_token: Option<String>,

    /// Create a new GitHub release automatically
    #[arg(long)]
    create_release: bool,

    /// Use latest GitHub release (overrides --github-release-id)
    #[arg(long)]
    use_latest_release: bool,

    /// GitHub release tag name (used when creating new release)
    #[arg(long, default_value = "v1.0.0")]
    release_tag: String,

    /// Comma-separated list of user IDs to send daily question to
    /// These users will receive the question via Zalo bot
    #[arg(long, value_delimiter = ',')]
    user_ids: Vec<String>,
    /// Include explanations when sending questions
    #[arg(long)]
    show_explanations: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("🚀 GMAT Zalo Bot Starting...");
    println!("📡 Fetching GMAT database...");

    let database = fetch_gmat_database().await?;

    if args.show_stats {
        show_database_stats(&database);
        return Ok(());
    }

    let require_image_upload = args.bot_service || !args.user_ids.is_empty();

    // Set up GitHub configuration if needed
    let github_config = if require_image_upload {
        setup_github_config(&args).await?
    } else {
        GitHubConfig {
            repo: String::new(),
            release_id: 0,
            token: String::new(),
        }
    };

    // Setup Zalo Bot Token
    let bot_token = if require_image_upload {
        args.bot_token
            .as_ref() // This gives you an Option<&String>
            .cloned() // This converts Option<&String> to Option<String> by cloning
            .or_else(|| env::var("ZALO_BOT_TOKEN").ok())
            .ok_or(
                "Bot token required. Set ZALO_BOT_TOKEN environment variable or use --bot-token",
            )?
    } else {
        String::new()
    };

    // Handle Zalo bot operations
    if args.bot_service {
        println!("\n🤖 Initializing Zalo Bot...");
        let zalo_bot = ZaloBot::new(bot_token);

        // Start continuous polling service
        println!("🚀 Starting bot service mode...");
        zalo_bot
            .start_polling_service(&database, &args.output_dir, &github_config)
            .await?;
    } else {
        // Process questions and generate images if needed
        let selected_questions = pick_random_questions(&database, &args.question_type, args.count);
        if selected_questions.is_empty() {
            return Err("No questions found matching your criteria.".into());
        }

        let zalo_bot = ZaloBot::new(bot_token);
        for (question_type, question_id) in selected_questions {
            if args.user_ids.is_empty() {
                render_question_to_image(
                    &fetch_question_content(&question_id)
                        .await
                        .expect(&format!("❌ Failed to fetch question {}", question_id)),
                    &question_type,
                    args.show_explanations,
                    &args.output_dir,
                )
                .await?;
            } else {
                send_question_to_users(
                    &zalo_bot,
                    &args.user_ids,
                    &question_id,
                    &question_type,
                    &args.output_dir,
                    &github_config,
                    args.show_explanations, // Respect CLI flag for explanations
                )
                .await?;
            }
        }
        println!("✅ Operation completed successfully!");
        return Ok(());
    }

    // Only show usage instructions if no action was taken
    if args.user_ids.is_empty() && !args.bot_service && !args.show_stats {
        println!("\n💡 Usage examples:");
        println!(
            "  # Start bot service (responds to each message automatically) with explanations"
        );
        println!("  cargo run -- --bot-service --question-type ps --show-explanations");
        println!();
        println!("  # Generate and send 3 PS questions to recent chats with explanations");
        println!("  cargo run -- --question-type ps --count 3 --send-zalo --show-explanations");
        println!();
        println!("  # Generate images locally without sending (includes explanations)");
        println!("  cargo run -- --question-type ds --generate-images --show-explanations");
        println!();
        println!("  # Show database statistics");
        println!("  cargo run -- --show-stats");
        println!();
        println!("🔧 Setup:");
        println!("  export ZALO_BOT_TOKEN=your_bot_token_here");
        println!("  export GITHUB_TOKEN=your_github_token_here  # Needs 'repo' scope");
        println!("  export GITHUB_REPOSITORY=your_repo_name");
        println!();
        println!("📦 GitHub Release Options:");
        println!("  cargo run -- --bot-service --create-release --release-tag v1.0.0");
        println!("  cargo run -- --bot-service --use-latest-release");
        println!("  cargo run -- --bot-service --github-release-id 123456");
        println!();
        println!("💡 The bot uploads question images to GitHub releases for hosting");
    }

    Ok(())
}
