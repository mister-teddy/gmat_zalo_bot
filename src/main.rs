use clap::Parser;
use gmat_zalo_bot::*;
use std::env;

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

    /// Custom caption for Zalo messages
    #[arg(long, default_value = "Here's your GMAT question! 📚")]
    caption: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    println!("🚀 GMAT Zalo Bot Starting...");
    println!("📡 Fetching GMAT database...");

    let database = match fetch_gmat_database().await {
        Ok(db) => db,
        Err(e) => {
            eprintln!("❌ Error fetching GMAT database: {}", e);
            std::process::exit(1);
        }
    };

    if args.show_stats {
        show_database_stats(&database);
        return;
    }

    let selected_questions = pick_random_questions(&database, &args.question_type, args.count);

    if selected_questions.is_empty() {
        println!("⚠️  No questions found matching your criteria.");
        return;
    }

    println!(
        "🎲 Selected {} Random Question{}:",
        selected_questions.len(),
        if selected_questions.len() > 1 {
            "s"
        } else {
            ""
        }
    );
    println!(
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    );

    let mut generated_images = Vec::new();

    for (i, (question_type, question_id)) in selected_questions.iter().enumerate() {
        println!(
            "\n{}. Question ID: {} ({})",
            i + 1,
            question_id,
            question_type
        );

        if args.generate_images || args.send_zalo {
            match fetch_question_content(question_id).await {
                Ok(content) => {
                    // Display basic question info
                    println!(
                        "  📝 Question preview: {}",
                        content.question.chars().take(100).collect::<String>()
                            + if content.question.len() > 100 {
                                "..."
                            } else {
                                ""
                            }
                    );

                    match render_question_to_image(&content, question_type, &args.output_dir).await
                    {
                        Ok(image_path) => {
                            generated_images.push((image_path, content, question_type.clone()));
                        }
                        Err(e) => {
                            eprintln!("  ❌ Failed to generate image: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("  ❌ Failed to fetch content: {}", e);
                }
            }
        }
    }

    println!(
        "\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    );

    if args.generate_images && !generated_images.is_empty() {
        println!(
            "🖼️  Generated {} image{}:",
            generated_images.len(),
            if generated_images.len() > 1 { "s" } else { "" }
        );
        for (image_path, _, _) in &generated_images {
            println!("   📁 {}", image_path);
        }
    }

    // Handle Zalo bot operations
    if args.send_zalo || args.bot_service {
        let bot_token = match args.bot_token.or_else(|| env::var("ZALO_BOT_TOKEN").ok()) {
            Some(token) => token,
            None => {
                eprintln!(
                    "❌ Bot token required. Set ZALO_BOT_TOKEN environment variable or use --bot-token"
                );
                eprintln!("💡 Example: export ZALO_BOT_TOKEN=your_bot_token_here");
                std::process::exit(1);
            }
        };

        println!("\n🤖 Initializing Zalo Bot...");
        let zalo_bot = ZaloBot::new(bot_token);

        if args.bot_service {
            // Start continuous polling service
            println!("🚀 Starting bot service mode...");
            if let Err(e) = zalo_bot
                .start_polling_service(
                    &database,
                    &args.question_type,
                    &args.output_dir,
                    &args.caption,
                )
                .await
            {
                eprintln!("❌ Bot service failed: {}", e);
                std::process::exit(1);
            }
        } else if args.send_zalo {
            // One-time send to recent chats
            if generated_images.is_empty() {
                eprintln!("❌ No images to send. Use --generate-images with --send-zalo");
                return;
            }

            println!("📱 Getting recent messages...");
            match zalo_bot.get_updates().await {
                Ok(messages) => {
                    if messages.is_empty() {
                        println!(
                            "⚠️  No recent messages found. Make sure users have sent messages to your bot recently."
                        );
                        return;
                    }

                    let mut chat_ids: Vec<String> =
                        messages.iter().map(|m| m.chat.id.clone()).collect();
                    chat_ids.sort();
                    chat_ids.dedup();

                    println!(
                        "📋 Found {} unique chat ID{}",
                        chat_ids.len(),
                        if chat_ids.len() > 1 { "s" } else { "" }
                    );

                    for (image_path, content, question_type) in &generated_images {
                        println!(
                            "\n📤 Sending question {} ({})...",
                            content.id, question_type
                        );

                        match encode_image_to_base64(image_path) {
                            Ok(base64_data_url) => {
                                let caption = format!(
                                    "{}\n\nQuestion ID: {} ({})",
                                    args.caption, content.id, question_type
                                );

                                for chat_id in &chat_ids {
                                    match zalo_bot
                                        .send_photo_base64(chat_id, &base64_data_url, &caption)
                                        .await
                                    {
                                        Ok(()) => {
                                            println!("  ✅ Sent to chat: {}", chat_id);
                                        }
                                        Err(e) => {
                                            eprintln!(
                                                "  ❌ Failed to send to chat {}: {}",
                                                chat_id, e
                                            );
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("  ❌ Failed to encode image: {}", e);
                            }
                        }
                    }

                    println!("\n🎉 Zalo sending completed!");
                }
                Err(e) => {
                    eprintln!("❌ Failed to get messages: {}", e);
                }
            }
        }
    }

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
    println!("  # Start service with custom caption");
    println!("  cargo run -- --bot-service --caption \"Daily GMAT practice! 📚\"");
    println!();
    println!("🔧 Setup: Set your bot token with: export ZALO_BOT_TOKEN=your_token_here");
}
