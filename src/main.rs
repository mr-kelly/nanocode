#![forbid(unsafe_code)]

mod llm;
mod tools;

use clap::{Parser, Subcommand};
use std::{io::{self, Read}, path::PathBuf};

const LOGO: &str = r#"
  ⚡ freecode
  ~300 lines · $0 · free forever
"#;

#[derive(Parser)]
#[command(
    name = "freecode",
    about = "Autonomous coding agent — always picks the best free model",
    long_about = None,
    version,
    disable_help_subcommand = true,
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Task to run (can also be passed as positional args)
    #[arg(trailing_var_arg = true)]
    prompt: Vec<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// List available free models ranked by weekly popularity
    ListFree,
}

#[tokio::main]
async fn main() {
    // Handle piped JSON input before clap parsing
    if !atty::is(atty::Stream::Stdin) {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf).expect("read stdin");
        #[derive(serde::Deserialize)]
        struct Input { cwd: Option<String>, prompt: String }
        let input: Input = serde_json::from_str(&buf)
            .unwrap_or_else(|_| Input { cwd: None, prompt: buf.trim().to_string() });
        let cwd = input.cwd.as_deref().map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap());
        if let Err(e) = llm::run(&cwd, &input.prompt).await {
            eprintln!("\n✗ {e:#}"); std::process::exit(1);
        }
        return;
    }

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::ListFree) => {
            if let Err(e) = llm::list_free_models().await {
                eprintln!("✗ {e:#}"); std::process::exit(1);
            }
        }
        None => {
            if cli.prompt.is_empty() {
                eprint!("{LOGO}");
                // print help
                use clap::CommandFactory;
                Cli::command().print_help().unwrap();
                println!();
                std::process::exit(0);
            }
            let task = cli.prompt.join(" ");
            let cwd = std::env::current_dir().unwrap();
            if let Err(e) = llm::run(&cwd, &task).await {
                eprintln!("\n✗ {e:#}"); std::process::exit(1);
            }
        }
    }
}
