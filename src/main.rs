use clap::Parser;
use gmat_zalo_bot::*;
use std::env;
use std::path::PathBuf;
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

/// Process questions and generate images if needed
async fn process_questions(
    questions: Vec<(QuestionType, String)>,
    output_dir: &str,
    generate_images: bool,
) -> Vec<(PathBuf, QuestionContent, QuestionType)> {
    let mut results = Vec::new();

    for (question_type, question_id) in questions {
        println!(
            "\n🔍 Processing question: {} ({})",
            question_id, question_type
        );

        if !generate_images {
            continue;
        }

        match fetch_question_content(&question_id).await {
            Ok(content) => {
                match render_question_to_image(&content, &question_type, true, output_dir).await {
                    Ok(image_path) => {
                        results.push((PathBuf::from(image_path), content, question_type));
                    }
                    Err(e) => {
                        eprintln!("  ❌ Failed to generate image: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("  ❌ Failed to fetch question content: {}", e);
            }
        }
    }

    results
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

                if !all_successful {
                    return Err("Failed to send to some users".into());
                }
                return Ok(());
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

    /// Generate images for the questions
    #[arg(long)]
    generate_images: bool,

    /// Output directory for generated images
    #[arg(long, default_value = "output")]
    output_dir: String,

    /// Send generated images via Zalo Bot API (one-time)
    #[arg(long)]
    send_zalo: bool,

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

    /// Send a daily question to a specific user (for use with GitHub Actions)
    #[arg(long)]
    daily_question: bool,

    /// Comma-separated list of user IDs to send daily question to (required with --daily-question)
    #[arg(long, value_delimiter = ',', default_value = "")]
    user_ids: Vec<String>,
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

    // Handle daily question command
    if args.daily_question {
        if args.user_ids.is_empty() {
            return Err("At least one user ID is required when using --daily-question".into());
        }

        let bot_token = args.bot_token.clone()
            .or_else(|| env::var("ZALO_BOT_TOKEN").ok())
            .ok_or("Zalo Bot token not found. Please set ZALO_BOT_TOKEN environment variable or use --bot-token")?;

        // Pick a random question
        let selected_questions = pick_random_questions(&database, &args.question_type, 1);
        if selected_questions.is_empty() {
            return Err("No questions found matching your criteria.".into());
        }

        let (question_type, question_id) = &selected_questions[0];
        println!(
            "📝 Selected question for daily challenge: {} ({})",
            question_id, question_type
        );

        let github_config = GitHubConfig {
            repo: String::new(),
            release_id: 0,
            token: String::new(),
        };

        let zalo_bot = ZaloBot::new(bot_token);
        send_question_to_users(
            &zalo_bot,
            &args.user_ids,
            question_id,
            question_type,
            &args.output_dir,
            &github_config,
            false, // Don't show explanations for daily questions
        )
        .await?;

        println!("✅ Successfully sent daily question to all users!");
        return Ok(());
    }

    // Process questions and generate images if needed
    let selected_questions = pick_random_questions(&database, &args.question_type, args.count);
    if selected_questions.is_empty() {
        return Err("No questions found matching your criteria.".into());
    }

    // Process images if needed (ignore result if we're not using it)
    let _generated_images = if args.generate_images || args.send_zalo {
        println!(
            "🎲 Selected {} random question{}:",
            selected_questions.len(),
            if selected_questions.len() > 1 {
                "s"
            } else {
                ""
            }
        );
        println!("{}", "-".repeat(80));
        let images = process_questions(
            selected_questions.clone(),
            &args.output_dir,
            args.generate_images,
        )
        .await;

        if !images.is_empty() {
            println!(
                "\n🖼️  Generated {} image{}:",
                images.len(),
                if images.len() > 1 { "s" } else { "" }
            );
            for (path, _, _) in &images {
                println!("   📁 {}", path.display());
            }
        }
        println!("{}", "-".repeat(80));
        images
    } else {
        Vec::new()
    };

    // Handle Zalo bot operations
    if args.send_zalo || args.bot_service {
        let bot_token = args
            .bot_token
            .as_ref() // This gives you an Option<&String>
            .cloned() // This converts Option<&String> to Option<String> by cloning
            .or_else(|| env::var("ZALO_BOT_TOKEN").ok())
            .ok_or(
                "Bot token required. Set ZALO_BOT_TOKEN environment variable or use --bot-token",
            )?;

        // Set up GitHub configuration if needed
        let github_config = if args.send_zalo || args.bot_service {
            setup_github_config(&args).await?
        } else {
            GitHubConfig {
                repo: String::new(),
                release_id: 0,
                token: String::new(),
            }
        };

        println!("\n🤖 Initializing Zalo Bot...");
        let zalo_bot = ZaloBot::new(bot_token);

        if args.bot_service {
            // Start continuous polling service
            println!("🚀 Starting bot service mode...");
            zalo_bot
                .start_polling_service(&database, &args.output_dir, &github_config)
                .await?;
        } else if args.send_zalo {
            // One-time send to recent chats
            if selected_questions.is_empty() {
                return Err("No questions selected to send".into());
            }

            println!("📱 Getting recent messages...");
            let messages = zalo_bot.get_updates().await?;

            if messages.is_empty() {
                return Err("No recent messages found. Make sure users have sent messages to your bot recently.".into());
            }

            let mut chat_ids: Vec<String> = messages.iter().map(|m| m.chat.id.clone()).collect();
            chat_ids.sort();
            chat_ids.dedup();

            println!(
                "📋 Found {} unique chat ID{}",
                chat_ids.len(),
                if chat_ids.len() > 1 { "s" } else { "" }
            );

            for (question_type, question_id) in selected_questions {
                println!(
                    "\n📤 Sending question {} ({})...",
                    question_id, question_type
                );

                for chat_id in &chat_ids {
                    if let Err(e) = send_question_to_users(
                        &zalo_bot,
                        &[chat_id.clone()],
                        &question_id,
                        &question_type,
                        &args.output_dir,
                        &github_config,
                        true, // Show explanations for manual sends
                    )
                    .await
                    {
                        eprintln!("  ❌ Failed to send to chat {}: {}", chat_id, e);
                    } else {
                        println!("  ✅ Sent to chat: {}", chat_id);
                    }
                }
            }

            println!("\n🎉 Zalo sending completed!");
        }
    }

    // Only show usage instructions if no action was taken
    if !args.daily_question && !args.send_zalo && !args.bot_service && !args.show_stats {
        println!("\n💡 Usage examples:");
        println!("  # Start bot service (responds to each message automatically)");
        println!("  cargo run -- --bot-service --question-type ps");
        println!();
        println!("  # Generate and send 3 PS questions to recent chats");
        println!("  cargo run -- --question-type ps --count 3 --send-zalo");
        println!();
        println!("  # Generate images locally without sending");
        println!("  cargo run -- --question-type ds --generate-images");
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
