use crate::database::{DatabaseError, Result};
/// Database schema definitions and migration helpers
use sqlx::SqlitePool;

pub async fn create_schema(pool: &SqlitePool) -> Result<()> {
    // Create jurisdictions table
    sqlx::query!(
        r#"
        CREATE TABLE IF NOT EXISTS jurisdictions (
            id INTEGER PRIMARY KEY,
            path TEXT UNIQUE NOT NULL,
            name TEXT NOT NULL,
            kind TEXT NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
        "#
    )
    .execute(pool)
    .await?;

    // Create elections table
    sqlx::query!(
        r#"
        CREATE TABLE IF NOT EXISTS elections (
            id INTEGER PRIMARY KEY,
            jurisdiction_id INTEGER NOT NULL,
            path TEXT NOT NULL,
            name TEXT NOT NULL,
            date DATE NOT NULL,
            data_format TEXT NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (jurisdiction_id) REFERENCES jurisdictions(id),
            UNIQUE(jurisdiction_id, path)
        )
        "#
    )
    .execute(pool)
    .await?;

    // Create contests table
    sqlx::query!(
        r#"
        CREATE TABLE IF NOT EXISTS contests (
            id INTEGER PRIMARY KEY,
            election_id INTEGER NOT NULL,
            office_id TEXT NOT NULL,
            office_name TEXT NOT NULL,
            jurisdiction_name TEXT,
            jurisdiction_code TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (election_id) REFERENCES elections(id),
            UNIQUE(election_id, office_id)
        )
        "#
    )
    .execute(pool)
    .await?;

    // Create candidates table
    sqlx::query!(
        r#"
        CREATE TABLE IF NOT EXISTS candidates (
            id INTEGER PRIMARY KEY,
            contest_id INTEGER NOT NULL,
            external_id TEXT,
            name TEXT NOT NULL,
            candidate_type TEXT NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (contest_id) REFERENCES contests(id)
        )
        "#
    )
    .execute(pool)
    .await?;

    // Create ballots table
    sqlx::query!(
        r#"
        CREATE TABLE IF NOT EXISTS ballots (
            id INTEGER PRIMARY KEY,
            contest_id INTEGER NOT NULL,
            ballot_id TEXT NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (contest_id) REFERENCES contests(id),
            UNIQUE(contest_id, ballot_id)
        )
        "#
    )
    .execute(pool)
    .await?;

    // Create ballot_choices table
    sqlx::query!(
        r#"
        CREATE TABLE IF NOT EXISTS ballot_choices (
            id INTEGER PRIMARY KEY,
            ballot_id INTEGER NOT NULL,
            rank_position INTEGER NOT NULL,
            choice_type TEXT NOT NULL,
            candidate_id INTEGER,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (ballot_id) REFERENCES ballots(id),
            FOREIGN KEY (candidate_id) REFERENCES candidates(id),
            UNIQUE(ballot_id, rank_position)
        )
        "#
    )
    .execute(pool)
    .await?;

    // Create raw_files table for tracking processed files
    sqlx::query!(
        r#"
        CREATE TABLE IF NOT EXISTS raw_files (
            id INTEGER PRIMARY KEY,
            election_id INTEGER NOT NULL,
            filename TEXT NOT NULL,
            file_hash TEXT NOT NULL,
            file_size INTEGER,
            processed_at TIMESTAMP,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (election_id) REFERENCES elections(id),
            UNIQUE(election_id, filename)
        )
        "#
    )
    .execute(pool)
    .await?;

    // Create indexes for performance
    create_indexes(pool).await?;

    Ok(())
}

async fn create_indexes(pool: &SqlitePool) -> Result<()> {
    let indexes = vec![
        "CREATE INDEX IF NOT EXISTS idx_elections_jurisdiction ON elections(jurisdiction_id)",
        "CREATE INDEX IF NOT EXISTS idx_contests_election ON contests(election_id)",
        "CREATE INDEX IF NOT EXISTS idx_candidates_contest ON candidates(contest_id)",
        "CREATE INDEX IF NOT EXISTS idx_ballots_contest ON ballots(contest_id)",
        "CREATE INDEX IF NOT EXISTS idx_ballot_choices_ballot ON ballot_choices(ballot_id)",
        "CREATE INDEX IF NOT EXISTS idx_ballot_choices_candidate ON ballot_choices(candidate_id)",
        "CREATE INDEX IF NOT EXISTS idx_raw_files_election ON raw_files(election_id)",
        "CREATE INDEX IF NOT EXISTS idx_raw_files_hash ON raw_files(file_hash)",
    ];

    for index_sql in indexes {
        sqlx::query(index_sql).execute(pool).await?;
    }

    Ok(())
}

/// Verify database schema integrity
pub async fn verify_schema(pool: &SqlitePool) -> Result<()> {
    // Check that all expected tables exist
    let tables =
        sqlx::query_scalar!("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .fetch_all(pool)
            .await?;

    let expected_tables = vec![
        "ballots",
        "ballot_choices",
        "candidates",
        "contests",
        "elections",
        "jurisdictions",
        "raw_files",
    ];

    for expected in &expected_tables {
        let expected_string = expected.to_string();
        if !tables
            .iter()
            .any(|t| t.as_ref().map_or(false, |name| name == &expected_string))
        {
            return Err(DatabaseError::Integrity(format!(
                "Missing table: {}",
                expected
            )));
        }
    }

    Ok(())
}
