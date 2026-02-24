#![forbid(unsafe_code)]

mod llm;
mod tools;

use anyhow::Result;
use serde::Deserialize;
use std::{io::{self, Read}, path::PathBuf};

#[derive(Debug, Deserialize)]
pub struct Input {
    pub cwd: Option<String>,
    pub prompt: String,
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let input = if !args.is_empty() {
        Input { cwd: None, prompt: args.join(" ") }
    } else if !atty::is(atty::Stream::Stdin) {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf).expect("read stdin");
        serde_json::from_str(&buf).unwrap_or_else(|_| Input { cwd: None, prompt: buf.trim().to_string() })
    } else {
        eprintln!("Usage: nanocode \"your task\"");
        std::process::exit(1);
    };

    let cwd = input.cwd.as_deref().map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    if let Err(e) = llm::run(&cwd, &input.prompt).await {
        eprintln!("\n✗ {e:#}");
        std::process::exit(1);
    }
}
