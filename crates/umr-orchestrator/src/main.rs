use std::path::PathBuf;
use std::process::exit;

use clap::{Parser, Subcommand};
use serde_json::json;

use umr_orchestrator::{find_intake_binary, RuntimeOrchestrator};

#[derive(Parser, Debug)]
#[command(name = "umr-orchestrator")]
#[command(about = "Universal Mobile Runtime core orchestrator and capability negotiator", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Ingest and defensively inspect a mobile package archive via sandboxed worker
    Inspect {
        /// Path to APK or IPA package
        package: PathBuf,
    },
    /// Evaluate package requirements against registered backend capabilities
    Negotiate {
        /// Path to APK or IPA package
        package: PathBuf,
    },
    /// List all registered execution backends and their operational modes
    ListBackends,
}

fn main() {
    let cli = Cli::parse();

    let intake_bin = match find_intake_binary() {
        Ok(bin) => bin,
        Err(err) => {
            eprintln!("Error locating intake worker: {}", err);
            exit(1);
        }
    };

    let orch = RuntimeOrchestrator::new(intake_bin);

    match cli.command {
        Commands::Inspect { package } => {
            match orch.inspect_package(&package) {
                Ok(metadata) => {
                    println!("{}", serde_json::to_string_pretty(&metadata).unwrap());
                    exit(0);
                }
                Err(err) => {
                    eprintln!("{}", serde_json::to_string_pretty(&err).unwrap());
                    exit(1);
                }
            }
        }
        Commands::Negotiate { package } => {
            match orch.inspect_package(&package) {
                Ok(metadata) => {
                    let result = orch.negotiate(&metadata);
                    let output = json!({
                        "package_id": metadata.package_id(),
                        "format": metadata.format,
                        "negotiation": result
                    });
                    println!("{}", serde_json::to_string_pretty(&output).unwrap());
                    exit(0);
                }
                Err(err) => {
                    eprintln!("{}", serde_json::to_string_pretty(&err).unwrap());
                    exit(1);
                }
            }
        }
        Commands::ListBackends => {
            let backends = orch.list_backends();
            println!("{}", serde_json::to_string_pretty(&backends).unwrap());
            exit(0);
        }
    }
}

