/// Performance metrics and benchmarking for database operations
use chrono::{DateTime, Utc};
use instant::Instant;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionMetrics {
    pub jurisdiction_path: String,
    pub election_path: String,
    pub contest_office: Option<String>,
    pub stage: IngestionStage,
    pub duration_ms: u64,
    pub ballots_processed: Option<u64>,
    pub files_processed: Option<u64>,
    pub memory_usage_mb: Option<f64>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IngestionStage {
    Discovery,
    FileReading,
    DatabaseInsertion,
    Validation,
    Complete,
}

impl std::fmt::Display for IngestionStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IngestionStage::Discovery => write!(f, "discovery"),
            IngestionStage::FileReading => write!(f, "file_reading"),
            IngestionStage::DatabaseInsertion => write!(f, "database_insertion"),
            IngestionStage::Validation => write!(f, "validation"),
            IngestionStage::Complete => write!(f, "complete"),
        }
    }
}

pub struct MetricsCollector {
    pool: SqlitePool,
    stage_timers: HashMap<String, Instant>,
}

impl MetricsCollector {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            stage_timers: HashMap::new(),
        }
    }

    /// Start timing a stage
    pub fn start_stage(&mut self, stage_key: &str) {
        self.stage_timers
            .insert(stage_key.to_string(), Instant::now());
    }

    /// End timing a stage and record metrics
    pub async fn end_stage(
        &mut self,
        stage_key: &str,
        jurisdiction_path: &str,
        election_path: &str,
        contest_office: Option<&str>,
        stage: IngestionStage,
        ballots_processed: Option<u64>,
        files_processed: Option<u64>,
    ) -> crate::database::Result<IngestionMetrics> {
        let duration = self
            .stage_timers
            .remove(stage_key)
            .map(|start| start.elapsed().as_millis() as u64)
            .unwrap_or(0);

        let metrics = IngestionMetrics {
            jurisdiction_path: jurisdiction_path.to_string(),
            election_path: election_path.to_string(),
            contest_office: contest_office.map(|s| s.to_string()),
            stage,
            duration_ms: duration,
            ballots_processed,
            files_processed,
            memory_usage_mb: get_memory_usage(),
            timestamp: Utc::now(),
        };

        // Store metrics in database
        self.store_metrics(&metrics).await?;

        Ok(metrics)
    }

    /// Store metrics in database
    async fn store_metrics(&self, metrics: &IngestionMetrics) -> crate::database::Result<()> {
        let stage_str = metrics.stage.to_string();
        let duration_ms = metrics.duration_ms as i64;
        let ballots_processed = metrics.ballots_processed.map(|b| b as i64);

        sqlx::query!(
            r#"
            INSERT INTO processing_metrics 
            (jurisdiction_path, election_path, contest_office, stage, duration_ms, ballots_processed, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            metrics.jurisdiction_path,
            metrics.election_path,
            metrics.contest_office,
            stage_str,
            duration_ms,
            ballots_processed,
            metrics.timestamp
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get metrics for a specific election
    pub async fn get_election_metrics(
        &self,
        jurisdiction_path: &str,
        election_path: &str,
    ) -> crate::database::Result<Vec<IngestionMetrics>> {
        let rows = sqlx::query!(
            r#"
            SELECT jurisdiction_path, election_path, contest_office, stage, 
                   duration_ms, ballots_processed, created_at
            FROM processing_metrics
            WHERE jurisdiction_path = ? AND election_path = ?
            ORDER BY created_at DESC
            "#,
            jurisdiction_path,
            election_path
        )
        .fetch_all(&self.pool)
        .await?;

        let metrics = rows
            .into_iter()
            .map(|row| {
                let stage = match row.stage.as_str() {
                    "discovery" => IngestionStage::Discovery,
                    "file_reading" => IngestionStage::FileReading,
                    "database_insertion" => IngestionStage::DatabaseInsertion,
                    "validation" => IngestionStage::Validation,
                    "complete" => IngestionStage::Complete,
                    _ => IngestionStage::Complete,
                };

                IngestionMetrics {
                    jurisdiction_path: row.jurisdiction_path,
                    election_path: row.election_path,
                    contest_office: row.contest_office,
                    stage,
                    duration_ms: row.duration_ms as u64,
                    ballots_processed: row.ballots_processed.map(|b| b as u64),
                    files_processed: None,
                    memory_usage_mb: None,
                    timestamp: Utc::now(), // TODO: Fix timestamp conversion
                }
            })
            .collect();

        Ok(metrics)
    }

    /// Print performance summary
    pub fn print_summary(&self, metrics: &[IngestionMetrics]) {
        use colored::*;

        println!(
            "\n{}",
            "📊 Ingestion Performance Summary".bright_cyan().bold()
        );
        println!("{}", "=".repeat(50).bright_cyan());

        let mut total_duration = 0u64;
        let mut total_ballots = 0u64;

        for metric in metrics {
            total_duration += metric.duration_ms;
            if let Some(ballots) = metric.ballots_processed {
                total_ballots += ballots;
            }

            let stage_color = match metric.stage {
                IngestionStage::Discovery => "yellow",
                IngestionStage::FileReading => "blue",
                IngestionStage::DatabaseInsertion => "green",
                IngestionStage::Validation => "magenta",
                IngestionStage::Complete => "bright_green",
            };

            println!(
                "{}: {} ms{}",
                format!("{:?}", metric.stage).color(stage_color),
                metric.duration_ms.to_string().bright_white(),
                if let Some(ballots) = metric.ballots_processed {
                    format!(" ({} ballots)", ballots.to_string().bright_yellow())
                } else {
                    String::new()
                }
            );
        }

        println!("{}", "-".repeat(50).bright_cyan());
        println!(
            "{}: {} ms",
            "Total Duration".bright_white().bold(),
            total_duration.to_string().bright_green().bold()
        );

        if total_ballots > 0 {
            println!(
                "{}: {}",
                "Total Ballots".bright_white().bold(),
                total_ballots.to_string().bright_green().bold()
            );

            let ballots_per_second = if total_duration > 0 {
                (total_ballots as f64 * 1000.0) / total_duration as f64
            } else {
                0.0
            };

            println!(
                "{}: {:.2} ballots/sec",
                "Processing Rate".bright_white().bold(),
                ballots_per_second.to_string().bright_green().bold()
            );
        }

        println!();
    }
}

/// Get current memory usage (simplified - in a real implementation you'd use a proper memory profiler)
fn get_memory_usage() -> Option<f64> {
    // This is a placeholder - in production you'd use something like:
    // - jemalloc stats
    // - /proc/self/status on Linux
    // - GetProcessMemoryInfo on Windows
    None
}

/// Create the processing_metrics table
pub async fn create_metrics_table(pool: &SqlitePool) -> crate::database::Result<()> {
    sqlx::query!(
        r#"
        CREATE TABLE IF NOT EXISTS processing_metrics (
            id INTEGER PRIMARY KEY,
            jurisdiction_path TEXT NOT NULL,
            election_path TEXT NOT NULL,
            contest_office TEXT,
            stage TEXT NOT NULL,
            duration_ms INTEGER NOT NULL,
            ballots_processed INTEGER,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
        "#
    )
    .execute(pool)
    .await?;

    // Create index for performance
    sqlx::query!(
        "CREATE INDEX IF NOT EXISTS idx_processing_metrics_election ON processing_metrics(jurisdiction_path, election_path)"
    )
    .execute(pool)
    .await?;

    Ok(())
}
