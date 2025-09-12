mod commands;
mod database;
mod formats;
mod model;
mod reports;
mod normalizers;
mod read_metadata;
mod report;
mod tabulator;
mod util;

use crate::commands::{discover, info, report, sync};
use crate::database::BallotsDatabase;
use crate::database::ingestion::BallotIngester;
use crate::reports::ReportsDatabase;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
struct Opts {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate and dump info about election.
    Info {
        /// Input directory to validate and dump.
        meta_dir: PathBuf,
    },
    /// Sync raw data files with metadata.
    Sync {
        /// Metadata directory
        meta_dir: PathBuf,
        /// Raw data directory
        raw_data_dir: PathBuf,
    },
    /// Discover contests from raw data files and generate metadata
    Discover {
        /// Raw data directory
        raw_data_dir: PathBuf,
        /// Metadata output directory
        meta_dir: PathBuf,
        /// Jurisdiction path (e.g., "us/ny/nyc")
        jurisdiction: String,
        /// Election path (e.g., "2025/07")
        election: String,
    },
    /// Ingest election data directly to SQLite database
    Ingest {
        /// Raw data directory
        raw_data_dir: PathBuf,
        /// SQLite database path
        database_path: PathBuf,
        /// Jurisdiction path (e.g., "us/ny/nyc")
        jurisdiction: String,
        /// Election path (e.g., "2025/07")
        election: String,
        /// Force re-ingestion even if data exists
        #[clap(long)]
        force: bool,
    },
    /// Generate reports database from ballots database
    GenerateReports {
        /// Ballots database path
        ballots_db_path: PathBuf,
        /// Reports database path
        reports_db_path: PathBuf,
    },
    /// Generate reports
    Report {
        /// Metadata directory
        meta_dir: PathBuf,
        /// Raw data directory
        raw_data_dir: PathBuf,
        /// Preprocessed file output directory
        preprocessed_dir: PathBuf,
        /// Report output directory
        report_dir: PathBuf,
        /// Whether to force preprocessing even if preprocessed files exist
        force_preprocess: bool,
        force_report: bool,
        /// Filter by jurisdiction path (e.g., "us/ny/nyc")
        #[clap(long)]
        jurisdiction: Option<String>,
        /// Filter by election path (e.g., "2025/07")
        #[clap(long)]
        election: Option<String>,
        /// Filter by contest office (e.g., "borough-president-manhattan")
        #[clap(long)]
        contest: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let opts = Opts::parse();

    match opts.command {
        Command::Info { meta_dir } => {
            info(&meta_dir);
        }
        Command::Sync {
            meta_dir,
            raw_data_dir,
        } => {
            sync(&meta_dir, &raw_data_dir);
        }
        Command::Discover {
            raw_data_dir,
            meta_dir,
            jurisdiction,
            election,
        } => {
            discover(&raw_data_dir, &meta_dir, &jurisdiction, &election);
        }
        Command::Ingest {
            raw_data_dir,
            database_path,
            jurisdiction,
            election,
            force,
        } => {
            if let Err(e) = ingest_election(
                &raw_data_dir,
                &database_path,
                &jurisdiction,
                &election,
                force,
            ).await {
                eprintln!("❌ Ingestion failed: {}", e);
                std::process::exit(1);
            }
        }
        Command::GenerateReports {
            ballots_db_path,
            reports_db_path,
        } => {
            if let Err(e) = generate_reports(&ballots_db_path, &reports_db_path).await {
                eprintln!("❌ Report generation failed: {}", e);
                std::process::exit(1);
            }
        }
        Command::Report {
            meta_dir,
            raw_data_dir,
            preprocessed_dir,
            report_dir,
            force_preprocess,
            force_report,
            jurisdiction,
            election,
            contest,
        } => {
            report(
                &meta_dir,
                &raw_data_dir,
                &report_dir,
                &preprocessed_dir,
                force_preprocess,
                force_report,
                jurisdiction.as_deref(),
                election.as_deref(),
                contest.as_deref(),
            );
        }
    }
}

/// Ingest election data directly to SQLite database
async fn ingest_election(
    raw_data_dir: &PathBuf,
    database_path: &PathBuf,
    jurisdiction: &str,
    election: &str,
    _force: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::database::{BallotsDatabase, ingestion::BallotIngester};
    use colored::*;

    println!(
        "🚀 Starting SQLite ingestion for {} {}",
        jurisdiction.bright_cyan(),
        election.bright_cyan()
    );

    // Step 1: Discover contests (reuse existing discovery logic)
    let raw_path = raw_data_dir;
    if !raw_path.exists() {
        return Err(format!("Raw data path does not exist: {}", raw_path.display()).into());
    }

    // For now, only support NYC format - extend this later
    if jurisdiction != "us/ny/nyc" {
        return Err(format!("Ingestion not yet implemented for jurisdiction: {}", jurisdiction).into());
    }

    // Discover contests using enhanced discovery
    let discovered_contests = discover_contests_for_ingestion(&raw_path)?;
    
    println!(
        "📋 Discovered {} contests",
        discovered_contests.len().to_string().bright_yellow()
    );

    // Step 2: Set up database
    let database_url = format!("sqlite:{}", database_path.display());
    let db = BallotsDatabase::new(&database_url).await?;
    
    println!("✅ Database initialized: {}", database_path.display().to_string().bright_green());

    // Step 3: Ingest data
    let mut ingester = BallotIngester::new(db);
    let summary = ingester.ingest_election(
        &raw_path,
        jurisdiction,
        election,
        &discovered_contests,
    ).await?;

    println!(
        "🎉 Ingestion completed! Processed {} ballots in {:.2} seconds",
        summary.total_ballots.to_string().bright_green().bold(),
        (summary.total_duration_ms as f64 / 1000.0).to_string().bright_green().bold()
    );

    Ok(())
}

/// Discover contests for ingestion using Python script
fn discover_contests_for_ingestion(
    raw_path: &std::path::Path,
) -> Result<Vec<crate::database::ingestion::DiscoveredContest>, Box<dyn std::error::Error>> {
    use crate::database::ingestion::DiscoveredContest;
    use std::collections::BTreeMap;
    use std::process::Command;

    println!("🔍 Discovering all NYC contests using Python script...");

    // Run the Python discovery script
    let output = Command::new("python3")
        .arg("discover_contests.py")
        .arg(raw_path.to_str().unwrap())
        .current_dir(std::env::current_dir()?)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Python discovery script failed: {}", stderr).into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Parse JSON output
    let discovery_result: serde_json::Value = serde_json::from_str(&stdout)?;
    let contests_json = discovery_result["contests"].as_array()
        .ok_or("Invalid JSON: missing contests array")?;

    let mut contests = Vec::new();
    
    for contest_json in contests_json {
        let office_id = contest_json["office_id"].as_str()
            .ok_or("Missing office_id")?.to_string();
        let office_name = contest_json["office_name"].as_str()
            .ok_or("Missing office_name")?.to_string();
        let jurisdiction_name = contest_json["jurisdiction_name"].as_str().map(|s| s.to_string());
        let jurisdiction_code = contest_json["jurisdiction_code"].as_str().map(|s| s.to_string());
        
        // Convert loader_params from JSON to BTreeMap
        let loader_params_json = &contest_json["loader_params"];
        let mut loader_params = BTreeMap::new();
        
        if let Some(obj) = loader_params_json.as_object() {
            for (key, value) in obj {
                if let Some(str_value) = value.as_str() {
                    loader_params.insert(key.clone(), str_value.to_string());
                }
            }
        }

        contests.push(DiscoveredContest {
            office_id,
            office_name,
            jurisdiction_name,
            jurisdiction_code,
            data_format: "us_ny_nyc".to_string(),
            loader_params,
        });
    }

    println!("✅ Discovered {} unique contests", contests.len());
    Ok(contests)
}


/// Generate reports database from ballots database
async fn generate_reports(
    ballots_db_path: &std::path::Path,
    reports_db_path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use colored::*;

    println!(
        "🚀 Generating reports database from {}",
        ballots_db_path.display().to_string().bright_cyan()
    );

    // Connect to ballots database
    let ballots_db_url = format!("sqlite:{}", ballots_db_path.display());
    let ballots_db = BallotsDatabase::new(&ballots_db_url).await?;

    // Connect to reports database (will create if doesn't exist)
    let reports_db_url = format!("sqlite:{}", reports_db_path.display());
    let reports_db = ReportsDatabase::new(&reports_db_url).await?;

    // Generate all reports
    reports_db.generate_reports_from_ballots(&ballots_db).await?;

    println!(
        "✅ Reports database created: {}",
        reports_db_path.display().to_string().bright_green()
    );

    Ok(())
}
