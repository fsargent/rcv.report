use crate::database::{BallotsDatabase, DatabaseError};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashMap;

pub mod generator;
pub mod tabulation;

#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),
    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("No data found for contest: {0}")]
    NoData(String),
}

pub type ReportResult<T> = std::result::Result<T, ReportError>;

/// Database for storing pre-computed reports
pub struct ReportsDatabase {
    pool: SqlitePool,
}

/// Election index entry for the main page
#[derive(Debug, Serialize, Deserialize)]
pub struct ElectionIndexEntry {
    pub path: String,
    pub jurisdiction_name: String,
    pub election_name: String,
    pub date: String,
    pub contests: Vec<ContestSummary>,
}

/// Contest summary for election listings
#[derive(Debug, Serialize, Deserialize)]
pub struct ContestSummary {
    pub office: String,
    pub office_name: String,
    pub name: String,
    pub winner: Option<String>,
    pub num_candidates: i64,
    pub num_rounds: i64,
    pub ballot_count: i64,
}

/// Full contest report matching the existing JSON format
#[derive(Debug, Serialize, Deserialize)]
pub struct ContestReport {
    pub info: ContestInfo,
    #[serde(rename = "ballotCount")]
    pub ballot_count: i64,
    pub candidates: Vec<CandidateInfo>,
    pub results: Vec<RoundResult>,
    pub summary: ResultSummary,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContestInfo {
    pub name: String,
    pub date: String,
    #[serde(rename = "dataFormat")]
    pub data_format: String,
    #[serde(rename = "jurisdictionPath")]
    pub jurisdiction_path: String,
    #[serde(rename = "electionPath")]
    pub election_path: String,
    pub office: String,
    #[serde(rename = "officeName")]
    pub office_name: String,
    #[serde(rename = "jurisdictionName")]
    pub jurisdiction_name: String,
    #[serde(rename = "electionName")]
    pub election_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CandidateInfo {
    pub name: String,
    pub candidate_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoundResult {
    pub round: i64,
    pub tally: HashMap<String, i64>,
    pub eliminated: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResultSummary {
    pub winner: Option<String>,
    pub total_rounds: i64,
    pub total_ballots: i64,
}

impl ReportsDatabase {
    pub async fn new(database_url: &str) -> ReportResult<Self> {
        let pool = SqlitePool::connect(database_url).await?;

        // Run reports migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| ReportError::Database(DatabaseError::Migration(e.to_string())))?;

        Ok(Self { pool })
    }

    /// Generate reports for all contests in the ballots database
    pub async fn generate_reports_from_ballots(
        &self,
        ballots_db: &BallotsDatabase,
    ) -> ReportResult<()> {
        println!("🚀 Generating reports database...");

        // Get all elections from ballots database
        let elections = ballots_db.get_all_elections().await?;

        for election in elections {
            println!(
                "📊 Processing election: {} {}",
                election.jurisdiction_path, election.election_path
            );

            // Insert election into reports index
            self.insert_election_index(&election).await?;

            // Get contests for this election
            let contests = ballots_db.get_contests_for_election(election.id).await?;

            for contest in contests {
                println!("  🏆 Processing contest: {}", contest.office_name);

                // Generate full contest report
                let report = generator::generate_contest_report(ballots_db, &contest).await?;

                // Insert contest summary
                let summary = ContestSummary {
                    office: contest.office.clone(),
                    office_name: contest.office_name.clone(),
                    name: contest.office_name.clone(),
                    winner: report.summary.winner.clone(),
                    num_candidates: report.candidates.len() as i64,
                    num_rounds: report.summary.total_rounds,
                    ballot_count: report.ballot_count,
                };

                self.insert_contest_summary(
                    &election.jurisdiction_path,
                    &election.election_path,
                    &summary,
                )
                .await?;

                // Insert full report
                let contest_path = format!(
                    "{}/{}/{}",
                    election.jurisdiction_path, election.election_path, contest.office
                );

                self.insert_contest_report(
                    &contest_path,
                    &election.jurisdiction_path,
                    &election.election_path,
                    &report,
                )
                .await?;
            }
        }

        println!("✅ Reports generation completed!");
        Ok(())
    }

    async fn insert_election_index(
        &self,
        election: &crate::database::ElectionInfo,
    ) -> ReportResult<()> {
        let election_path = format!("{}/{}", election.jurisdiction_path, election.election_path);
        let date_str = election.date.format("%Y-%m-%d").to_string();

        sqlx::query!(
            r#"
            INSERT OR REPLACE INTO election_index (path, jurisdiction_name, election_name, date)
            VALUES (?, ?, ?, ?)
            "#,
            election_path,
            election.jurisdiction_name,
            election.name,
            date_str
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn insert_contest_summary(
        &self,
        jurisdiction_path: &str,
        election_path: &str,
        summary: &ContestSummary,
    ) -> ReportResult<()> {
        let election_full_path = format!("{}/{}", jurisdiction_path, election_path);

        sqlx::query!(
            r#"
            INSERT OR REPLACE INTO contest_summaries 
            (election_path, office, office_name, name, winner, num_candidates, num_rounds, ballot_count)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            election_full_path,
            summary.office,
            summary.office_name,
            summary.name,
            summary.winner,
            summary.num_candidates,
            summary.num_rounds,
            summary.ballot_count
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn insert_contest_report(
        &self,
        contest_path: &str,
        jurisdiction_path: &str,
        election_path: &str,
        report: &ContestReport,
    ) -> ReportResult<()> {
        let election_full_path = format!("{}/{}", jurisdiction_path, election_path);
        let report_json = serde_json::to_string(report)?;

        sqlx::query!(
            r#"
            INSERT OR REPLACE INTO contest_reports 
            (path, election_path, office, report_json, ballot_count, winner)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
            contest_path,
            election_full_path,
            report.info.office,
            report_json,
            report.ballot_count,
            report.summary.winner
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get election index for the main page
    pub async fn get_election_index(&self) -> ReportResult<Vec<ElectionIndexEntry>> {
        let elections = sqlx::query!(
            r#"
            SELECT path, jurisdiction_name, election_name, date
            FROM election_index
            ORDER BY date DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut result = Vec::new();

        for election in elections {
            let contests = sqlx::query_as!(
                ContestSummary,
                r#"
                SELECT office, office_name, name, winner, num_candidates, num_rounds, ballot_count
                FROM contest_summaries
                WHERE election_path = ?
                ORDER BY office_name
                "#,
                election.path
            )
            .fetch_all(&self.pool)
            .await?;

            result.push(ElectionIndexEntry {
                path: election.path,
                jurisdiction_name: election.jurisdiction_name,
                election_name: election.election_name,
                date: election.date,
                contests,
            });
        }

        Ok(result)
    }

    /// Get a specific contest report
    pub async fn get_contest_report(&self, contest_path: &str) -> ReportResult<ContestReport> {
        let row = sqlx::query!(
            r#"
            SELECT report_json
            FROM contest_reports
            WHERE path = ?
            "#,
            contest_path
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let report: ContestReport = serde_json::from_str(&row.report_json)?;
                Ok(report)
            }
            None => Err(ReportError::NoData(contest_path.to_string())),
        }
    }
}
