use crate::database::metrics::{IngestionStage, MetricsCollector};
/// High-performance ballot ingestion with benchmarking
use crate::database::{BallotsDatabase, DatabaseError, Result};
use crate::formats;
use crate::model::election::{CandidateType, Choice, Election};
use colored::*;
use std::collections::HashMap;
use std::path::Path;

pub struct BallotIngester {
    db: BallotsDatabase,
    metrics: MetricsCollector,
}

#[derive(Debug, Clone)]
pub struct DiscoveredContest {
    pub office_id: String,
    pub office_name: String,
    pub jurisdiction_name: Option<String>,
    pub jurisdiction_code: Option<String>,
    pub data_format: String,
    pub loader_params: std::collections::BTreeMap<String, String>,
}

#[derive(Debug)]
pub struct IngestionSummary {
    pub jurisdiction_path: String,
    pub election_path: String,
    pub contests_processed: usize,
    pub total_ballots: u64,
    pub total_duration_ms: u64,
    pub ballots_per_second: f64,
}

impl BallotIngester {
    pub fn new(db: BallotsDatabase) -> Self {
        let metrics = MetricsCollector::new(db.pool().clone());
        Self { db, metrics }
    }

    /// Main ingestion entry point
    pub async fn ingest_election(
        &mut self,
        raw_data_path: &Path,
        jurisdiction_path: &str,
        election_path: &str,
        discovered_contests: &[DiscoveredContest],
    ) -> Result<IngestionSummary> {
        println!(
            "🚀 Starting ingestion for {} {}",
            jurisdiction_path.bright_cyan(),
            election_path.bright_cyan()
        );

        let total_start_key = format!("total_{}_{}", jurisdiction_path, election_path);
        self.metrics.start_stage(&total_start_key);

        // Step 1: Set up jurisdiction and election
        self.metrics.start_stage("setup");
        let (_jurisdiction_id, election_id) = self
            .setup_jurisdiction_and_election(jurisdiction_path, election_path, discovered_contests)
            .await?;

        self.metrics
            .end_stage(
                "setup",
                jurisdiction_path,
                election_path,
                None,
                IngestionStage::Discovery,
                None,
                Some(discovered_contests.len() as u64),
            )
            .await?;

        // Step 2: Process each contest
        let mut total_ballots = 0u64;
        let mut contests_processed = 0usize;

        for contest in discovered_contests {
            println!(
                "  📊 Processing contest: {}",
                contest.office_name.bright_yellow()
            );

            let contest_ballots = self
                .ingest_contest(
                    raw_data_path,
                    jurisdiction_path,
                    election_path,
                    election_id,
                    contest,
                )
                .await?;

            total_ballots += contest_ballots;
            contests_processed += 1;

            println!(
                "    ✅ Processed {} ballots for {}",
                contest_ballots.to_string().bright_green(),
                contest.office_name
            );
        }

        // Step 3: Finalize and collect metrics
        let total_metrics = self
            .metrics
            .end_stage(
                &total_start_key,
                jurisdiction_path,
                election_path,
                None,
                IngestionStage::Complete,
                Some(total_ballots),
                Some(contests_processed as u64),
            )
            .await?;

        let summary = IngestionSummary {
            jurisdiction_path: jurisdiction_path.to_string(),
            election_path: election_path.to_string(),
            contests_processed,
            total_ballots,
            total_duration_ms: total_metrics.duration_ms,
            ballots_per_second: if total_metrics.duration_ms > 0 {
                (total_ballots as f64 * 1000.0) / total_metrics.duration_ms as f64
            } else {
                0.0
            },
        };

        self.print_ingestion_summary(&summary);
        Ok(summary)
    }

    /// Set up jurisdiction and election records
    async fn setup_jurisdiction_and_election(
        &mut self,
        jurisdiction_path: &str,
        election_path: &str,
        discovered_contests: &[DiscoveredContest],
    ) -> Result<(i64, i64)> {
        // Extract jurisdiction info from path
        let (jurisdiction_name, jurisdiction_kind) =
            self.parse_jurisdiction_info(jurisdiction_path);

        // Insert/update jurisdiction
        let jurisdiction_id = self
            .db
            .upsert_jurisdiction(jurisdiction_path, &jurisdiction_name, &jurisdiction_kind)
            .await?;

        // Insert/update election (using first contest's data format)
        let data_format = discovered_contests
            .first()
            .map(|c| c.data_format.as_str())
            .unwrap_or("unknown");

        let election_id = self
            .db
            .upsert_election(
                jurisdiction_id,
                election_path,
                "Primary Election", // TODO: Extract from discovery
                "2025-06-24",       // TODO: Extract from discovery
                data_format,
            )
            .await?;

        Ok((jurisdiction_id, election_id))
    }

    /// Ingest a single contest
    async fn ingest_contest(
        &mut self,
        raw_data_path: &Path,
        jurisdiction_path: &str,
        election_path: &str,
        election_id: i64,
        contest: &DiscoveredContest,
    ) -> Result<u64> {
        let contest_key = format!("contest_{}_{}", jurisdiction_path, contest.office_id);

        // Step 1: Insert contest record
        let contest_id = self
            .db
            .insert_contest(
                election_id,
                &contest.office_id,
                &contest.office_name,
                contest.jurisdiction_name.as_deref(),
                contest.jurisdiction_code.as_deref(),
            )
            .await?;

        // Step 2: Read ballot data using existing format readers
        self.metrics.start_stage(&format!("{}_read", contest_key));

        let election_data = self.read_contest_ballots(raw_data_path, contest)?;

        self.metrics
            .end_stage(
                &format!("{}_read", contest_key),
                jurisdiction_path,
                election_path,
                Some(&contest.office_id),
                IngestionStage::FileReading,
                Some(election_data.ballots.len() as u64),
                None,
            )
            .await?;

        // Step 3: Insert ballot data into database
        self.metrics.start_stage(&format!("{}_insert", contest_key));

        let ballot_count = self
            .insert_election_data(contest_id, &election_data)
            .await?;

        self.metrics
            .end_stage(
                &format!("{}_insert", contest_key),
                jurisdiction_path,
                election_path,
                Some(&contest.office_id),
                IngestionStage::DatabaseInsertion,
                Some(ballot_count),
                None,
            )
            .await?;

        Ok(ballot_count)
    }

    /// Read ballot data using existing format readers
    fn read_contest_ballots(
        &self,
        raw_data_path: &Path,
        contest: &DiscoveredContest,
    ) -> Result<Election> {
        match contest.data_format.as_str() {
            "us_ny_nyc" => {
                let election = formats::us_ny_nyc::nyc_ballot_reader(
                    raw_data_path,
                    contest.loader_params.clone(),
                );
                Ok(election)
            }
            "nist_sp_1500" => {
                // TODO: Implement when needed
                Err(DatabaseError::Integrity(
                    "NIST SP 1500 format not yet implemented".to_string(),
                ))
            }
            _ => Err(DatabaseError::Integrity(format!(
                "Unsupported format: {}",
                contest.data_format
            ))),
        }
    }

    /// Insert election data into database with transaction
    async fn insert_election_data(&self, contest_id: i64, election: &Election) -> Result<u64> {
        let mut tx = self.db.pool().begin().await?;

        // Insert candidates
        let mut candidate_map = HashMap::new();
        for (idx, candidate) in election.candidates.iter().enumerate() {
            let external_id = idx.to_string();
            let candidate_type_str = match candidate.candidate_type {
                CandidateType::Regular => "regular",
                CandidateType::WriteIn => "write_in",
                CandidateType::QualifiedWriteIn => "qualified_write_in",
            };

            // Try to insert candidate, ignore if exists
            sqlx::query!(
                r#"
                INSERT OR IGNORE INTO candidates (contest_id, external_id, name, candidate_type)
                VALUES (?, ?, ?, ?)
                "#,
                contest_id,
                external_id,
                candidate.name,
                candidate_type_str
            )
            .execute(&mut *tx)
            .await?;

            // Get the candidate ID (whether newly inserted or existing)
            let candidate_id = sqlx::query!(
                r#"
                SELECT id FROM candidates 
                WHERE contest_id = ? AND external_id = ?
                "#,
                contest_id,
                external_id
            )
            .fetch_one(&mut *tx)
            .await?
            .id;

            candidate_map.insert(idx, candidate_id);
        }

        // Insert ballots and choices in batches for performance
        let batch_size = 1000;
        let mut ballot_count = 0u64;

        for batch in election.ballots.chunks(batch_size) {
            for ballot in batch {
                // Insert ballot (ignore duplicates)
                sqlx::query!(
                    r#"
                    INSERT OR IGNORE INTO ballots (contest_id, ballot_id)
                    VALUES (?, ?)
                    "#,
                    contest_id,
                    ballot.id
                )
                .execute(&mut *tx)
                .await?;

                // Get the ballot ID (whether newly inserted or existing)
                let ballot_db_id = sqlx::query!(
                    r#"
                    SELECT id FROM ballots 
                    WHERE contest_id = ? AND ballot_id = ?
                    "#,
                    contest_id,
                    ballot.id
                )
                .fetch_one(&mut *tx)
                .await?
                .id;

                // Insert ballot choices
                for (rank, choice) in ballot.choices.iter().enumerate() {
                    let (choice_type, candidate_id) = match choice {
                        Choice::Vote(candidate_id) => {
                            // Find the database candidate_id for this CandidateId
                            let candidate_idx = candidate_id.0 as usize;
                            ("candidate", candidate_map.get(&candidate_idx).copied())
                        }
                        Choice::Undervote => ("undervote", None),
                        Choice::Overvote => ("overvote", None),
                    };

                    let rank_position = (rank + 1) as i64; // 1-based ranking

                    sqlx::query!(
                        r#"
                        INSERT OR IGNORE INTO ballot_choices (ballot_id, rank_position, choice_type, candidate_id)
                        VALUES (?, ?, ?, ?)
                        "#,
                        ballot_db_id,
                        rank_position,
                        choice_type,
                        candidate_id
                    )
                    .execute(&mut *tx)
                    .await?;
                }

                ballot_count += 1;
            }
        }

        tx.commit().await?;
        Ok(ballot_count)
    }

    /// Parse jurisdiction information from path
    fn parse_jurisdiction_info(&self, jurisdiction_path: &str) -> (String, String) {
        // Parse paths like "us/ny/nyc" -> ("New York City", "city")
        match jurisdiction_path {
            "us/ny/nyc" => ("New York City".to_string(), "city".to_string()),
            "us/ca/sfo" => ("San Francisco".to_string(), "city".to_string()),
            "us/me" => ("Maine".to_string(), "state".to_string()),
            _ => ("Unknown".to_string(), "unknown".to_string()),
        }
    }

    /// Print ingestion summary
    fn print_ingestion_summary(&self, summary: &IngestionSummary) {
        println!("\n{}", "🎉 Ingestion Complete!".bright_green().bold());
        println!("{}", "=".repeat(50).bright_green());
        println!(
            "{}: {} {}",
            "Election".bright_white().bold(),
            summary.jurisdiction_path.bright_cyan(),
            summary.election_path.bright_cyan()
        );
        println!(
            "{}: {}",
            "Contests Processed".bright_white().bold(),
            summary.contests_processed.to_string().bright_yellow()
        );
        println!(
            "{}: {}",
            "Total Ballots".bright_white().bold(),
            summary.total_ballots.to_string().bright_yellow()
        );
        println!(
            "{}: {} ms",
            "Total Duration".bright_white().bold(),
            summary.total_duration_ms.to_string().bright_yellow()
        );
        println!(
            "{}: {:.2} ballots/sec",
            "Processing Rate".bright_white().bold(),
            summary.ballots_per_second.to_string().bright_green().bold()
        );
        println!();
    }
}
