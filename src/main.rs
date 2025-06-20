use clap::{Parser, ValueEnum};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
#[command(about = "Pick random GMAT questions from the database")]
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
        all_questions.insert(QuestionType::RC, &self.reading_comprehension);
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

fn pick_random_questions(
    database: &GmatDatabase,
    question_type: &Option<QuestionType>,
    count: usize,
) -> Vec<(QuestionType, String)> {
    let mut rng = rand::thread_rng();
    let mut results = Vec::new();

    match question_type {
        Some(qtype) => {
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
    println!("Fetching GMAT database...");

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
        "🎲 Random Question{} Selected:",
        if args.count > 1 { "s" } else { "" }
    );
    println!(
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    );

    for (i, (question_type, question_id)) in selected_questions.iter().enumerate() {
        if args.count > 1 {
            println!(
                "{}. Question ID: {} ({})",
                i + 1,
                question_id,
                question_type
            );
        } else {
            println!("Question ID: {}", question_id);
            println!("Category: {}", question_type);
        }
    }

    if args.count > 1 {
        println!(
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        );
        println!(
            "Total: {} question{} selected",
            args.count,
            if args.count > 1 { "s" } else { "" }
        );
    }

    println!("\n💡 Usage examples:");
    println!("  cargo run -- --question-type ps --count 3    # Pick 3 Problem Solving questions");
    println!(
        "  cargo run -- --question-type rc             # Pick 1 Reading Comprehension question"
    );
    println!("  cargo run -- --count 5                      # Pick 5 questions from any category");
    println!("  cargo run -- --show-stats                   # Show database statistics");
}
