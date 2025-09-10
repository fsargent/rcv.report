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

/// Discover contests for ingestion (enhanced version of existing discover)
fn discover_contests_for_ingestion(
    _raw_path: &std::path::Path,
) -> Result<Vec<crate::database::ingestion::DiscoveredContest>, Box<dyn std::error::Error>> {
    use crate::database::ingestion::DiscoveredContest;
    use std::collections::BTreeMap;

    // For NYC format, reuse the existing discovery logic but return our format
    let mut contests = Vec::new();

    // This is a simplified version - in practice, we'd enhance the existing discover command
    // to return structured data instead of writing JSON files
    
    // For now, create a sample contest for testing
    let mut loader_params = BTreeMap::new();
    loader_params.insert("candidatesFile".to_string(), "Primary Election 2025 - 06-24-2025_CandidacyID_To_Name.xlsx".to_string());
    loader_params.insert("cvrPattern".to_string(), "2025P1V.+\\.xlsx".to_string());
    loader_params.insert("jurisdictionName".to_string(), "Citywide".to_string());
    loader_params.insert("officeName".to_string(), "DEM Borough President - Manhattan".to_string());

    contests.push(DiscoveredContest {
        office_id: "borough-president-manhattan".to_string(),
        office_name: "DEM Borough President - Manhattan".to_string(),
        jurisdiction_name: Some("Manhattan".to_string()),
        jurisdiction_code: Some("026918".to_string()),
        data_format: "us_ny_nyc".to_string(),
        loader_params,
    });

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
