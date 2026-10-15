use std::path::PathBuf;
use std::process::exit;

use clap::Parser;
use serde_json::json;

use umr_intake_worker::inspect_package;

#[derive(Parser, Debug)]
#[command(name = "umr-intake-worker")]
#[command(about = "Isolated unprivileged mobile application package inspector for UMR", long_about = None)]
struct Cli {
    /// Path to candidate APK or IPA package file
    #[arg(short, long)]
    inspect: PathBuf,

    /// Output full machine-readable JSON on stdout
    #[arg(long, default_value_t = true)]
    json: bool,
}

fn main() {
    let cli = Cli::parse();

    match inspect_package(&cli.inspect) {
        Ok(metadata) => {
            if cli.json {
                let output = json!({
                    "status": "ok",
                    "metadata": metadata
                });
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            } else {
                println!("Package accepted for inspection:");
                println!("  Format:   {}", metadata.format);
                println!("  ID:       {}", metadata.package_id());
                println!("  Size:     {} bytes", metadata.file_size_bytes);
                println!("  SHA-256:  {}", metadata.sha256_hash);
            }
            exit(0);
        }
        Err(err) => {
            if cli.json {
                let output = json!({
                    "status": "error",
                    "error": err
                });
                eprintln!("{}", serde_json::to_string_pretty(&output).unwrap());
            } else {
                eprintln!("Package inspection failed [{}]: {}", err.code, err.message);
                if let Some(sug) = &err.suggestion {
                    eprintln!("  Suggestion: {}", sug);
                }
            }
            exit(1);
        }
    }
}

