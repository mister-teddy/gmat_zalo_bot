use clap::{Parser, ValueEnum};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ValueEnum)]
enum QuestionType {
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

#[derive(Parser, Debug)]
#[command(name = "gmat-question-picker")]
#[command(about = "Pick random GMAT questions from the database and render them as images")]
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
}

#[derive(Debug, Deserialize, Serialize)]
struct GmatDatabase {
    #[serde(rename = "RC")]
    reading_comprehension: Vec<String>,
    #[serde(rename = "SC")]
    sentence_correction: Vec<String>,
    #[serde(rename = "CR")]
    critical_reasoning: Vec<String>,
    #[serde(rename = "PS")]
    problem_solving: Vec<String>,
    #[serde(rename = "DS")]
    data_sufficiency: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct QuestionContent {
    id: String,
    src: String,
    explanations: Vec<String>,
    #[serde(rename = "type")]
    question_type: String,
    question: String,
    answers: Vec<String>,
}

impl GmatDatabase {
    fn get_questions_by_type(&self, question_type: &QuestionType) -> &Vec<String> {
        match question_type {
            QuestionType::RC => &self.reading_comprehension,
            QuestionType::SC => &self.sentence_correction,
            QuestionType::CR => &self.critical_reasoning,
            QuestionType::PS => &self.problem_solving,
            QuestionType::DS => &self.data_sufficiency,
        }
    }

    fn get_all_questions(&self) -> HashMap<QuestionType, &Vec<String>> {
        let mut all_questions = HashMap::new();
        // Exclude RC questions as they have a different JSON structure
        all_questions.insert(QuestionType::SC, &self.sentence_correction);
        all_questions.insert(QuestionType::CR, &self.critical_reasoning);
        all_questions.insert(QuestionType::PS, &self.problem_solving);
        all_questions.insert(QuestionType::DS, &self.data_sufficiency);
        all_questions
    }

    fn total_questions(&self) -> usize {
        self.reading_comprehension.len()
            + self.sentence_correction.len()
            + self.critical_reasoning.len()
            + self.problem_solving.len()
            + self.data_sufficiency.len()
    }
}

async fn fetch_gmat_database() -> Result<GmatDatabase, Box<dyn std::error::Error>> {
    let url = "https://mister-teddy.github.io/gmat-database/index.json";
    let response = reqwest::get(url).await?;
    let database: GmatDatabase = response.json().await?;
    Ok(database)
}

async fn fetch_question_content(
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

fn pick_random_questions(
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

fn generate_html_content(content: &QuestionContent, question_type: &QuestionType) -> String {
    let type_color = match question_type {
        QuestionType::RC => "#e74c3c",
        QuestionType::SC => "#3498db",
        QuestionType::CR => "#2ecc71",
        QuestionType::PS => "#f39c12",
        QuestionType::DS => "#0068ff",
    };

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

fn check_wkhtmltoimage() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("wkhtmltoimage").arg("--version").output();

    match output {
        Ok(_) => Ok(()),
        Err(_) => Err("wkhtmltoimage not found. Please install wkhtmltopdf package which includes wkhtmltoimage.".into())
    }
}

async fn render_question_to_image(
    content: &QuestionContent,
    question_type: &QuestionType,
    output_dir: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir)?;

    // Generate HTML content
    let html_content = generate_html_content(content, question_type);

    // Create temporary directory for HTML file
    let temp_dir = TempDir::new()?;
    let html_file = temp_dir.path().join(format!("{}.html", content.id));
    let output_file = Path::new(output_dir).join(format!("question_{}.png", content.id));

    // Write HTML to temporary file
    fs::write(&html_file, html_content)?;

    println!("  🖼️  Rendering question {} to image...", content.id);

    // Use wkhtmltoimage to convert HTML to PNG
    let output = Command::new("wkhtmltoimage")
        .args([
            "--width",
            "1200",
            "--height",
            "0", // Auto height
            "--format",
            "png",
            "--quality",
            "95",
            "--enable-javascript",
            "--javascript-delay",
            "2000", // Wait for MathJax to render
            "--load-error-handling",
            "ignore",
            "--load-media-error-handling",
            "ignore",
        ])
        .arg(&html_file)
        .arg(&output_file)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("wkhtmltoimage failed: {}", stderr).into());
    }

    let output_path = output_file.to_string_lossy().to_string();
    println!("  ✅ Image saved: {}", output_path);

    Ok(output_path)
}

fn show_database_stats(database: &GmatDatabase) {
    println!("📊 GMAT Database Statistics:");
    println!(
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    );

    let stats = [
        (
            "RC",
            "Reading Comprehension",
            database.reading_comprehension.len(),
        ),
        (
            "SC",
            "Sentence Correction",
            database.sentence_correction.len(),
        ),
        (
            "CR",
            "Critical Reasoning",
            database.critical_reasoning.len(),
        ),
        ("PS", "Problem Solving", database.problem_solving.len()),
        ("DS", "Data Sufficiency", database.data_sufficiency.len()),
    ];

    for (code, name, count) in stats {
        println!("  {:<2} │ {:<20} │ {:>4} questions", code, name, count);
    }

    println!(
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    );
    println!("     Total: {} questions", database.total_questions());
    println!();
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    println!("🎯 GMAT Question Picker");

    // Check if wkhtmltoimage is available when image generation is requested
    if args.generate_images {
        if let Err(e) = check_wkhtmltoimage() {
            eprintln!("❌ {}", e);
            eprintln!("💡 Install wkhtmltopdf:");
            eprintln!("   • macOS: brew install wkhtmltopdf");
            eprintln!("   • Ubuntu: sudo apt-get install wkhtmltopdf");
            eprintln!("   • Windows: Download from https://wkhtmltopdf.org/downloads.html");
            std::process::exit(1);
        }
    }

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

        if args.generate_images {
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
                            generated_images.push(image_path);
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
        for image_path in generated_images {
            println!("   📁 {}", image_path);
        }
    }

    println!("\n💡 Usage examples:");
    println!(
        "  cargo run -- --question-type ps --count 3 --generate-images    # Pick 3 PS questions and generate images"
    );
    println!(
        "  cargo run -- --question-type ds --generate-images              # Pick 1 DS question and generate image"
    );
    println!(
        "  cargo run -- --count 5 --output-dir ./my-questions             # Pick 5 questions, save to custom directory"
    );
    println!(
        "  cargo run -- --show-stats                                      # Show database statistics"
    );
}
