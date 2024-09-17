use dialoguer::{theme::ColorfulTheme, Select};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::{self, File};
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use termimad::minimad;

#[derive(Debug, Serialize, Deserialize)]
struct Part {
    text: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiRequest {
    contents: Vec<Content>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Candidate {
    content: Content,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

fn create_entry(path: &Path) -> Result<(), std::io::Error> {
    let mut file = match File::create(path) {
        Ok(file) => file,
        Err(err) => return Err(err),
    };

    file.write_all(
        r#"### work

### studies"#
            .as_bytes(),
    )
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let mut diary_dir = env::var("DIARY_DIR").unwrap();
    let gemini_api_key = env::var("GEMINI_API_KEY").unwrap();
    let selections = &["create", "summary"];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .default(0)
        .items(&selections[..])
        .interact()
        .unwrap();

    let entry_paths: Vec<PathBuf> = fs::read_dir(&diary_dir)?
        .map(|res| res.map(|e| e.path()).unwrap())
        .collect();

    let entries = entry_paths
        .clone()
        .into_iter()
        .map(|entry| entry.file_name().unwrap().to_str().unwrap().to_owned())
        .collect::<Vec<_>>();

    if selections[selection] == "summary" {
        if entries.is_empty() {
            eprintln!("No entries found");
            return Ok(());
        }

        let entry_selection = Select::with_theme(&ColorfulTheme::default())
            .default(0)
            .items(&entries[..])
            .interact()
            .unwrap();

        let entry_content = fs::read_to_string(entry_paths[entry_selection].clone()).unwrap();
        let mut prompt =
            String::from("Write brief a summary in first person of only what was done in the work section of this entry in brazilian portuguese: ");
        prompt.push_str(&entry_content);

        let client = reqwest::Client::new();
        let response = client
            .post("https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash-latest:generateContent")
            .query(&[("key", gemini_api_key)])
            .json(&GeminiRequest{
                contents: vec![Content{
                    parts: vec![Part{
                        text: prompt
                    }]
                }]
            })
            .send().await.unwrap().json::<GeminiResponse>().await.unwrap();

        let summary = &response
            .candidates
            .first()
            .unwrap()
            .content
            .parts
            .first()
            .unwrap()
            .text;

        let mut text = String::from("# Work summary for ");
        text.push_str(entries.get(entry_selection).unwrap());
        text.push('\n');
        text.push_str(summary);

        let options = minimad::Options::default()
            .clean_indentations(false)
            .continue_spans(true);
        let parsed_text = minimad::parse_text(&text, options);

        let skin = termimad::get_default_skin();
        let fmt_text = termimad::FmtText::from_text(skin, parsed_text, None);
        println!("{fmt_text}");

        return Ok(());
    }

    let mut today = chrono::Local::now().format("%Y-%m-%d").to_string();
    today.push_str(".md");

    if entries.contains(&today) {
        println!("Today's entry already exists");
        return Ok(());
    };

    diary_dir.push_str(&today);

    let path = Path::new(diary_dir.as_str());

    match create_entry(path) {
        Ok(_) => {
            println!("entry {} created", today);
            return Ok(());
        }
        Err(err) => {
            eprintln!("{}", err);
        }
    };

    Ok(())
}
