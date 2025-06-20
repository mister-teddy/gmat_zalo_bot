use clap::ValueEnum;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

const BOT_API_URL: &str = "https://dev-bot-api.zapps.vn";
const LONG_POLLING_TIMEOUT: u64 = 6 * 60 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ValueEnum)]
pub enum QuestionType {
    /// Reading Comprehension
    RC,
    /// Sentence Correction
    SC,
    /// Critical Reasoning
    CR,
    /// Problem Solving
    PS,
    /// Data Sufficiency
    DS,
}

impl std::fmt::Display for QuestionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuestionType::RC => write!(f, "Reading Comprehension"),
            QuestionType::SC => write!(f, "Sentence Correction"),
            QuestionType::CR => write!(f, "Critical Reasoning"),
            QuestionType::PS => write!(f, "Problem Solving"),
            QuestionType::DS => write!(f, "Data Sufficiency"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GmatDatabase {
    #[serde(rename = "RC")]
    pub reading_comprehension: Vec<String>,
    #[serde(rename = "SC")]
    pub sentence_correction: Vec<String>,
    #[serde(rename = "CR")]
    pub critical_reasoning: Vec<String>,
    #[serde(rename = "PS")]
    pub problem_solving: Vec<String>,
    #[serde(rename = "DS")]
    pub data_sufficiency: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct QuestionContent {
    pub id: String,
    pub src: String,
    pub explanations: Vec<String>,
    #[serde(rename = "type")]
    pub question_type: String,
    pub question: String,
    pub answers: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ZaloMessage {
    pub sender: ZaloSender,
    pub chat: ZaloChat,
    pub text: Option<String>,
    pub photo_url: Option<String>,
    pub caption: Option<String>,
    pub message_id: String,
    pub date: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ZaloSender {
    pub id: String,
    pub is_bot: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ZaloChat {
    pub id: String,
    pub chat_type: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ZaloUpdate {
    pub message: Option<ZaloMessage>,
    pub event_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ZaloUpdatesResult {
    Single(ZaloUpdate),
    Multiple(Vec<ZaloUpdate>),
    Empty(serde_json::Value),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ZaloUpdatesResponse {
    pub ok: bool,
    pub result: ZaloUpdatesResult,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ZaloSendPhotoResponse {
    pub ok: bool,
    pub result: ZaloSendResult,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ZaloSendResult {
    pub message_id: String,
    pub date: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ZaloSendMessageResponse {
    pub ok: bool,
    pub result: ZaloSendResult,
}

pub struct ZaloBot {
    pub bot_token: String,
    pub client: reqwest::Client,
}

impl GmatDatabase {
    pub fn get_questions_by_type(&self, question_type: &QuestionType) -> &Vec<String> {
        match question_type {
            QuestionType::RC => &self.reading_comprehension,
            QuestionType::SC => &self.sentence_correction,
            QuestionType::CR => &self.critical_reasoning,
            QuestionType::PS => &self.problem_solving,
            QuestionType::DS => &self.data_sufficiency,
        }
    }

    pub fn get_all_questions(&self) -> HashMap<QuestionType, &Vec<String>> {
        let mut all_questions = HashMap::new();
        // Exclude RC questions as they have a different JSON structure
        all_questions.insert(QuestionType::SC, &self.sentence_correction);
        all_questions.insert(QuestionType::CR, &self.critical_reasoning);
        all_questions.insert(QuestionType::PS, &self.problem_solving);
        all_questions.insert(QuestionType::DS, &self.data_sufficiency);
        all_questions
    }

    pub fn total_questions(&self) -> usize {
        self.reading_comprehension.len()
            + self.sentence_correction.len()
            + self.critical_reasoning.len()
            + self.problem_solving.len()
            + self.data_sufficiency.len()
    }
}

impl ZaloBot {
    pub fn new(bot_token: String) -> Self {
        Self {
            bot_token,
            client: reqwest::Client::new(),
        }
    }

    pub async fn get_updates(&self) -> Result<Vec<ZaloMessage>, Box<dyn std::error::Error>> {
        let url = format!("{}/bot{}/getUpdates", BOT_API_URL, self.bot_token);

        println!("🌐 Making API request to: {}", url);
        println!("📤 Request payload: {{\"timeout\": {LONG_POLLING_TIMEOUT}}}");

        let response = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "timeout": LONG_POLLING_TIMEOUT,
            }))
            .timeout(std::time::Duration::from_secs(LONG_POLLING_TIMEOUT + 5)) // 24 hours + 5 seconds
            .send()
            .await?;

        let status = response.status();
        println!("📥 Response status: {}", status);

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            println!("❌ Error response body: {}", error_text);
            return Err(format!("Failed to get updates: {} - {}", status, error_text).into());
        }

        let response_text = response.text().await?;

        // Always log the response for debugging
        println!("🔍 DEBUG - Full API Response:");
        println!("----------------------------------------");
        println!("{}", response_text);
        println!("----------------------------------------");
        println!("📏 Response length: {} bytes", response_text.len());

        // Try to pretty print the JSON for better readability
        if let Ok(parsed_json) = serde_json::from_str::<serde_json::Value>(&response_text) {
            if let Ok(pretty_json) = serde_json::to_string_pretty(&parsed_json) {
                println!("🎨 Pretty JSON:");
                println!("----------------------------------------");
                println!("{}", pretty_json);
                println!("----------------------------------------");
            }
        }

        let updates: ZaloUpdatesResponse = serde_json::from_str(&response_text).map_err(|e| {
            format!(
                "Failed to parse JSON response: {}\n\nRaw response: {}\n\nError details: {:?}",
                e, response_text, e
            )
        })?;

        println!("✅ Successfully parsed response: ok={}", updates.ok);

        if !updates.ok {
            return Err(format!("API returned error: {}", response_text).into());
        }

        let mut messages = Vec::new();

        match updates.result {
            ZaloUpdatesResult::Single(update) => {
                println!(
                    "📝 Received single update with event: {}",
                    update.event_name
                );
                if let Some(message) = update.message {
                    println!(
                        "💬 Message from user: {} in chat: {}",
                        message.sender.id, message.chat.id
                    );
                    messages.push(message);
                }
            }
            ZaloUpdatesResult::Multiple(update_list) => {
                println!("📝 Received {} updates", update_list.len());
                for (i, update) in update_list.iter().enumerate() {
                    println!("  Update {}: event={}", i + 1, update.event_name);
                    if let Some(message) = &update.message {
                        println!(
                            "    Message from user: {} in chat: {}",
                            message.sender.id, message.chat.id
                        );
                    }
                }
                for update in update_list {
                    if let Some(message) = update.message {
                        messages.push(message);
                    }
                }
            }
            ZaloUpdatesResult::Empty(value) => {
                println!("📝 Received empty/unknown result: {:?}", value);
            }
        }

        println!("📊 Total messages extracted: {}", messages.len());
        Ok(messages)
    }

    pub async fn start_polling_service(
        &self,
        database: &GmatDatabase,
        question_type: &Option<QuestionType>,
        output_dir: &str,
        caption: &str,
        github_config: &GitHubConfig,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔄 Starting long polling service...");
        println!("📱 Bot is now listening for messages. Send any message to get a GMAT question!");
        println!("🛑 Press Ctrl+C to stop the bot");

        loop {
            tokio::select! {
                // Handle shutdown signal
                _ = tokio::signal::ctrl_c() => {
                    println!("\n🛑 Received shutdown signal. Stopping bot gracefully...");
                    break;
                }

                // Handle API updates
                result = self.get_updates() => {
                    match result {
                        Ok(messages) => {
                            if !messages.is_empty() {
                                println!("\n📨 Received {} new message(s)", messages.len());

                                for message in messages {
                                    self.handle_message(
                                        &message,
                                        database,
                                        question_type,
                                        output_dir,
                                        caption,
                                        github_config,
                                    )
                                    .await;
                                }
                            } else {
                                println!("⏳ No new messages (normal for long polling)");
                            }
                        }
                        Err(e) => {
                            eprintln!("⚠️  Error getting updates: {}", e);

                            // Check if it's a timeout (normal for long polling) or a real error
                            if e.to_string().contains("timeout") {
                                println!("🔄 24-hour polling timeout, continuing...");
                            } else {
                                println!("🔄 Error occurred, retrying in 5 seconds...");
                                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                            }
                        }
                    }
                }
            }
        }

        println!("✅ Bot stopped successfully");
        Ok(())
    }

    async fn handle_message(
        &self,
        message: &ZaloMessage,
        database: &GmatDatabase,
        question_type: &Option<QuestionType>,
        output_dir: &str,
        caption: &str,
        github_config: &GitHubConfig,
    ) {
        let chat_id = &message.chat.id;
        let sender_id = &message.sender.id;

        let message_text = message.text.as_deref().unwrap_or("").trim().to_lowercase();

        println!(
            "💬 Processing message '{}' from user: {} in chat: {}",
            message_text, sender_id, chat_id
        );

        // Parse message to determine question type
        let requested_type = match message_text.as_str() {
            "rc" | "reading comprehension" => Some(QuestionType::RC),
            "sc" | "sentence correction" => Some(QuestionType::SC),
            "cr" | "critical reasoning" => Some(QuestionType::CR),
            "ps" | "problem solving" => Some(QuestionType::PS),
            "ds" | "data sufficiency" => Some(QuestionType::DS),
            _ => None,
        };

        if let Some(q_type) = requested_type {
            // User requested a specific question type
            println!("🎯 User requested {} questions", q_type);

            // Pick a random question of the requested type
            let selected_questions = pick_random_questions(database, &Some(q_type), 1);

            if selected_questions.is_empty() {
                let error_msg = format!(
                    "⚠️ Sorry, no {} questions are available at the moment. Please try another type.",
                    q_type
                );
                if let Err(e) = self.send_message(chat_id, &error_msg).await {
                    eprintln!("❌ Failed to send error message: {}", e);
                }
                return;
            }

            let (selected_type, question_id) = &selected_questions[0];
            println!("🎯 Selected question: {} ({})", question_id, selected_type);

            // Fetch question content
            match fetch_question_content(question_id).await {
                Ok(content) => {
                    // Generate image
                    match render_question_to_image(&content, selected_type, output_dir).await {
                        Ok(image_path) => {
                            // Upload to GitHub and send
                            let full_caption = format!(
                                "{}\n\nQuestion ID: {} ({})\nFrom: {}",
                                caption, content.id, selected_type, content.src
                            );

                            match self
                                .send_photo_from_file(
                                    chat_id,
                                    &image_path,
                                    &full_caption,
                                    github_config,
                                )
                                .await
                            {
                                Ok(()) => {
                                    println!(
                                        "✅ Successfully sent {} question {} to user {}",
                                        selected_type, question_id, sender_id
                                    );
                                }
                                Err(e) => {
                                    eprintln!(
                                        "❌ Failed to send photo to user {}: {}",
                                        sender_id, e
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!(
                                "❌ Failed to generate image for question {}: {}",
                                question_id, e
                            );
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to fetch question {}: {}", question_id, e);
                }
            }
        } else {
            // User message doesn't match any question type, send help message
            let help_message = format!(
                "Hello! 👋 I'm your GMAT practice bot.\n\n\
                To get a question, please send one of these types:\n\n\
                📖 **RC** - Reading Comprehension\n\
                ✏️ **SC** - Sentence Correction\n\
                🧠 **CR** - Critical Reasoning\n\
                🔢 **PS** - Problem Solving\n\
                📊 **DS** - Data Sufficiency\n\n\
                Just type the abbreviation (like 'PS' or 'ds') to get a random question of that type!"
            );

            match self.send_message(chat_id, &help_message).await {
                Ok(()) => {
                    println!(
                        "💡 Sent help message to user {} (unrecognized input: '{}')",
                        sender_id, message_text
                    );
                }
                Err(e) => {
                    eprintln!(
                        "❌ Failed to send help message to user {}: {}",
                        sender_id, e
                    );
                }
            }
        }
    }

    pub async fn send_photo(
        &self,
        chat_id: &str,
        photo_url: &str,
        caption: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!("{}/bot{}/sendPhoto", BOT_API_URL, self.bot_token);

        let response = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "chat_id": chat_id,
                "photo_url": photo_url,
                "caption": caption
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Failed to send photo: {} - {}", status, error_text).into());
        }

        let _result: ZaloSendPhotoResponse = response.json().await?;
        println!("  ✅ Photo sent successfully to chat: {}", chat_id);
        Ok(())
    }

    pub async fn send_photo_from_file(
        &self,
        chat_id: &str,
        image_path: &str,
        caption: &str,
        github_config: &GitHubConfig,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Upload to GitHub release first, then send the URL
        let github_url = upload_to_github_release(
            &github_config.repo,
            github_config.release_id,
            &github_config.token,
            image_path,
        )
        .await?;
        self.send_photo(chat_id, &github_url, caption).await
    }

    pub async fn send_message(
        &self,
        chat_id: &str,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!("{}/bot{}/sendMessage", BOT_API_URL, self.bot_token);

        let response = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "chat_id": chat_id,
                "text": text
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Failed to send message: {} - {}", status, error_text).into());
        }

        let _result: ZaloSendMessageResponse = response.json().await?;
        println!("  ✅ Message sent successfully to chat: {}", chat_id);
        Ok(())
    }
}

pub async fn fetch_gmat_database() -> Result<GmatDatabase, Box<dyn std::error::Error>> {
    let url = "https://mister-teddy.github.io/gmat-database/index.json";
    let response = reqwest::get(url).await?;
    let database: GmatDatabase = response.json().await?;
    Ok(database)
}

pub async fn fetch_question_content(
    question_id: &str,
) -> Result<QuestionContent, Box<dyn std::error::Error>> {
    let url = format!(
        "https://mister-teddy.github.io/gmat-database/{}.json",
        question_id
    );
    println!("  📥 Fetching question content for ID: {}", question_id);

    let response = reqwest::get(&url).await?;
    if !response.status().is_success() {
        return Err(format!(
            "Failed to fetch question {}: {}",
            question_id,
            response.status()
        )
        .into());
    }

    let content: QuestionContent = response.json().await?;
    Ok(content)
}

pub fn pick_random_questions(
    database: &GmatDatabase,
    question_type: &Option<QuestionType>,
    count: usize,
) -> Vec<(QuestionType, String)> {
    let mut rng = rand::thread_rng();
    let mut results = Vec::new();

    match question_type {
        Some(qtype) => {
            // Skip RC questions as they have a different JSON structure
            if *qtype == QuestionType::RC {
                eprintln!(
                    "⚠️  RC questions are currently not supported due to different JSON structure"
                );
                return results;
            }

            let questions = database.get_questions_by_type(qtype);
            let selected: Vec<_> = questions
                .choose_multiple(&mut rng, count.min(questions.len()))
                .cloned()
                .collect();

            for question_id in selected {
                results.push((qtype.clone(), question_id));
            }
        }
        None => {
            // Pick from all question types randomly
            let all_questions = database.get_all_questions();
            let mut all_items = Vec::new();

            for (qtype, questions) in all_questions {
                for question_id in questions {
                    all_items.push((qtype, question_id.clone()));
                }
            }

            let selected: Vec<_> = all_items
                .choose_multiple(&mut rng, count.min(all_items.len()))
                .cloned()
                .collect();

            results.extend(selected);
        }
    }

    results
}

pub fn generate_html_content(content: &QuestionContent, question_type: &QuestionType) -> String {
    let type_color = "#0068ff";

    let answers_html = if !content.answers.is_empty() {
        let options = content
            .answers
            .iter()
            .enumerate()
            .map(|(i, answer)| {
                let label = match i {
                    0 => "A",
                    1 => "B",
                    2 => "C",
                    3 => "D",
                    4 => "E",
                    _ => &format!("{}", i + 1),
                };
                format!(
                    "<div class=\"answer-option\"><strong>{})</strong> {}</div>",
                    label, answer
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"
        <div class="answers-section">
            <h3>Answer Choices:</h3>
            {}
        </div>
        "#,
            options
        )
    } else {
        String::new()
    };

    let explanations_html = if !content.explanations.is_empty() {
        let explanations = content
            .explanations
            .iter()
            .enumerate()
            .map(|(i, explanation)| {
                format!(
                    "<div class=\"explanation\"><h4>Explanation {}:</h4>{}</div>",
                    i + 1,
                    explanation
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"
        <div class="explanations-section">
            <h3>Explanations:</h3>
            {}
        </div>
        "#,
            explanations
        )
    } else {
        String::new()
    };

    format!(
        r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>GMAT Question {}</title>
    <script id="MathJax-script" async src="https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-mml-chtml.js"></script>
    <script>
        window.MathJax = {{
            tex: {{
                inlineMath: [['\\(', '\\)'], ['$', '$']],
                displayMath: [['\\[', '\\]'], ['$$', '$$']]
            }},
            options: {{
                processHtmlClass: 'tex2jax_process',
                processEscapes: true
            }}
        }};
    </script>
    <style>
        body {{
            font-family: Georgia, 'Times New Roman', Times, serif;
            max-width: 1000px;
            margin: 0 auto;
            padding: 30px;
            line-height: 1.6;
            background-color: #ffffff;
            color: #333;
        }}

        .question-header {{
            background: {};
            color: white;
            padding: 25px;
            border-radius: 8px;
            margin-bottom: 30px;
        }}

        .question-id {{
            font-size: 1.1em;
            font-weight: 600;
            opacity: 0.9;
            margin-bottom: 5px;
        }}

        .question-type {{
            font-size: 1.8em;
            font-weight: 700;
            margin: 0;
        }}

        .question-content {{
            background: white;
            padding: 30px;
            margin-bottom: 25px;
        }}

        .question-text {{
            font-size: 1.2em;
            line-height: 1.7;
            margin-bottom: 25px;
            color: #2c3e50;
        }}

        .answers-section {{
            background: #f9f9f9;
            padding: 25px;
            margin-bottom: 25px;
        }}

        .answers-section h3 {{
            color: {};
            margin-top: 0;
            margin-bottom: 20px;
            font-size: 1.3em;
        }}

        .answer-option {{
            padding: 12px 15px;
            margin: 8px 0;
            background: white;
            font-size: 1.1em;
        }}

        .explanations-section {{
            background: white;
            padding: 25px;
        }}

        .explanations-section h3 {{
            color: {};
            margin-top: 0;
            margin-bottom: 20px;
            font-size: 1.3em;
        }}

        .explanation {{
            margin-bottom: 25px;
            padding: 20px;
            background: #f9f9f9;
        }}

        .explanation h4 {{
            color: {};
            margin-top: 0;
            margin-bottom: 15px;
        }}

        .source-link {{
            margin-top: 30px;
            padding: 15px;
            background: #f9f9f9;
            font-size: 0.9em;
        }}

        .source-link a {{
            color: {};
            text-decoration: none;
        }}

        .source-link a:hover {{
            text-decoration: underline;
        }}

        /* LaTeX Math styling */
        .MathJax {{
            font-size: 1.1em !important;
        }}

        /* Table styling for better readability */
        table {{
            border-collapse: collapse;
            width: 100%;
            margin: 15px 0;
        }}

        th, td {{
            padding: 12px;
            text-align: left;
            border-bottom: 1px solid #eee;
        }}

        th {{
            background-color: #f9f9f9;
            font-weight: bold;
        }}

        /* List styling */
        ul, ol {{
            padding-left: 25px;
        }}

        li {{
            margin: 8px 0;
        }}

        /* Code blocks */
        code {{
            background-color: #f9f9f9;
            padding: 2px 6px;
            font-family: 'Courier New', monospace;
        }}

        /* Emphasis */
        strong {{
            color: #2c3e50;
        }}

        em {{
            color: #7f8c8d;
        }}
    </style>
</head>
<body>
    <div class="question-header">
        <div class="question-id">Question ID: {}</div>
        <h1 class="question-type">{}</h1>
    </div>

    <div class="question-content">
        <div class="question-text tex2jax_process">
            {}
        </div>

        {}

        {}
    </div>

    <div class="source-link">
        <strong>Source:</strong> <a href="{}" target="_blank">{}</a>
    </div>
</body>
</html>
    "#,
        content.id,
        type_color, // header background
        type_color, // answers section title
        type_color, // explanations section title
        type_color, // explanation title
        type_color, // source link
        content.id,
        question_type,
        content.question,
        answers_html,
        explanations_html,
        content.src,
        content.src
    )
}

pub fn check_wkhtmltoimage() -> Result<(), Box<dyn std::error::Error>> {
    match Command::new("wkhtmltoimage").arg("--version").output() {
        Ok(_) => Ok(()),
        Err(_) => Err("wkhtmltoimage is not installed or not in PATH. Please install it first. Visit: https://wkhtmltopdf.org/downloads.html".into()),
    }
}

pub async fn render_question_to_image(
    content: &QuestionContent,
    question_type: &QuestionType,
    output_dir: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    check_wkhtmltoimage()?;

    let html_content = generate_html_content(content, question_type);

    // Create a temporary directory for the HTML file
    let temp_dir = TempDir::new()?;
    let html_path = temp_dir.path().join("question.html");

    // Write HTML to temporary file
    fs::write(&html_path, html_content)?;

    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir)?;

    // Generate output path
    let output_path = Path::new(output_dir).join(format!("question_{}.png", content.id));

    println!("  🖼️  Rendering question to image...");

    // Run wkhtmltoimage command
    let output = Command::new("wkhtmltoimage")
        .arg("--format")
        .arg("jpg")
        .arg("--width")
        .arg("1200")
        .arg("--disable-smart-width")
        .arg("--quality")
        .arg("70")
        .arg(html_path.to_str().unwrap())
        .arg(&output_path)
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "wkhtmltoimage failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    println!("  ✅ Image saved: {}", output_path.display());
    Ok(output_path.to_string_lossy().to_string())
}

pub fn show_database_stats(database: &GmatDatabase) {
    println!("\n📊 GMAT Database Statistics:");
    println!(
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    );
    println!(
        "📖 Reading Comprehension (RC): {} questions",
        database.reading_comprehension.len()
    );
    println!(
        "✏️  Sentence Correction (SC):   {} questions",
        database.sentence_correction.len()
    );
    println!(
        "🧠 Critical Reasoning (CR):    {} questions",
        database.critical_reasoning.len()
    );
    println!(
        "🔢 Problem Solving (PS):       {} questions",
        database.problem_solving.len()
    );
    println!(
        "📊 Data Sufficiency (DS):      {} questions",
        database.data_sufficiency.len()
    );
    println!(
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    );
    println!("🎯 Total Questions: {}", database.total_questions());
    println!("⚠️  Note: RC questions are currently not supported due to different JSON structure");
    println!();
}

#[derive(Debug)]
pub struct GitHubConfig {
    pub repo: String,
    pub release_id: u64,
    pub token: String,
}

#[derive(Debug, Deserialize)]
struct GitHubReleaseResponse {
    upload_url: String,
}

#[derive(Debug, Deserialize)]
struct GitHubAssetResponse {
    browser_download_url: String,
}

pub async fn create_github_release(
    repo: &str,
    token: &str,
    tag_name: &str,
) -> Result<u64, Box<dyn std::error::Error>> {
    println!("  🏷️  Creating GitHub release with tag: {}", tag_name);

    let client = reqwest::Client::new();
    let url = format!("https://api.github.com/repos/{}/releases", repo);

    let release_data = serde_json::json!({
        "tag_name": tag_name,
        "name": format!("GMAT Bot Images - {}", tag_name),
        "body": "Automated release for GMAT question images",
        "draft": false,
        "prerelease": false
    });

    let response = client
        .post(&url)
        .header("Authorization", format!("token {}", token))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "gmat-zalo-bot")
        .json(&release_data)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Failed to create release: {} - {}", status, error_text).into());
    }

    let release_response: serde_json::Value = response.json().await?;
    let release_id = release_response["id"]
        .as_u64()
        .ok_or("Failed to get release ID from response")?;

    println!("  ✅ Created release with ID: {}", release_id);
    Ok(release_id)
}

pub async fn get_latest_release_id(
    repo: &str,
    token: &str,
) -> Result<u64, Box<dyn std::error::Error>> {
    println!("  🔍 Getting latest release ID...");

    let client = reqwest::Client::new();
    let url = format!("https://api.github.com/repos/{}/releases/latest", repo);

    let response = client
        .get(&url)
        .header("Authorization", format!("token {}", token))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "gmat-zalo-bot")
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Failed to get latest release: {} - {}", status, error_text).into());
    }

    let release_response: serde_json::Value = response.json().await?;
    let release_id = release_response["id"]
        .as_u64()
        .ok_or("Failed to get release ID from response")?;

    println!("  ✅ Found latest release ID: {}", release_id);
    Ok(release_id)
}

pub async fn upload_to_github_release(
    repo: &str,
    release_id: u64,
    token: &str,
    image_path: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    println!("  📤 Uploading image to GitHub release...");

    let client = reqwest::Client::new();

    // First, get the release info to obtain the upload_url
    println!("  🔍 Getting release upload URL...");
    let release_url = format!(
        "https://api.github.com/repos/{}/releases/{}",
        repo, release_id
    );

    let release_response = client
        .get(&release_url)
        .header("Authorization", format!("token {}", token))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "gmat-zalo-bot")
        .send()
        .await?;

    if !release_response.status().is_success() {
        let status = release_response.status();
        let error_text = release_response.text().await.unwrap_or_default();

        if status == 404 {
            return Err(format!(
                "Release not found. Please create a release first or use --github-release-id with a valid release ID.\n\
                You can create a release manually on GitHub or the bot can auto-create one.\n\
                Repository: {}, Release ID: {}",
                repo, release_id
            ).into());
        }

        return Err(format!("Failed to get release info: {} - {}", status, error_text).into());
    }

    let release_info: GitHubReleaseResponse = release_response.json().await?;

    // Extract the base upload URL (remove the {?name,label} template part)
    let upload_url = release_info
        .upload_url
        .split('{')
        .next()
        .unwrap_or(&release_info.upload_url);

    // Read the image file
    let file_bytes = fs::read(image_path)?;
    println!("  📏 Image size: {} bytes", file_bytes.len());

    // Generate unique filename based on timestamp and question ID
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let path = Path::new(image_path);
    let base_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("question");
    let file_name = format!("{}__{}.png", base_name, timestamp);

    // Upload the asset using the upload_url
    let upload_url_with_name = format!("{}?name={}", upload_url, file_name);
    println!("  📤 Uploading {} to GitHub...", file_name);

    let response = client
        .post(&upload_url_with_name)
        .header("Authorization", format!("token {}", token))
        .header("Content-Type", "image/png")
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "gmat-zalo-bot")
        .body(file_bytes)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();

        if status == 422 {
            return Err(format!(
                "Asset upload failed - likely due to duplicate filename: {}\n\
                GitHub returns 422 when an asset with the same name already exists.\n\
                Error details: {}",
                file_name, error_text
            )
            .into());
        }

        return Err(format!(
            "GitHub upload failed: {} - {}\n\
            Make sure your GitHub token has the 'repo' scope and write access to the repository.",
            status, error_text
        )
        .into());
    }

    let github_response: GitHubAssetResponse = response.json().await?;

    println!(
        "  ✅ Image uploaded to GitHub: {}",
        github_response.browser_download_url
    );
    Ok(github_response.browser_download_url)
}
