pub mod ingestion;
pub mod metrics;
pub mod schema;

use crate::model::election::CandidateType;
use sqlx::SqlitePool;

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("SQLite error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Migration error: {0}")]
    Migration(String),
    #[error("Data integrity error: {0}")]
    Integrity(String),
}

pub type Result<T> = std::result::Result<T, DatabaseError>;

#[derive(Clone)]
pub struct BallotsDatabase {
    pool: SqlitePool,
}

impl BallotsDatabase {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = SqlitePool::connect(database_url).await?;

        // TODO: Run migrations - for now, assume database is already set up
        // sqlx::migrate!("./migrations")
        //     .run(&pool)
        //     .await
        //     .map_err(|e| DatabaseError::Migration(e.to_string()))?;

        Ok(Self { pool })
    }

    pub async fn create_in_memory() -> Result<Self> {
        Self::new("sqlite::memory:").await
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Insert or get jurisdiction ID
    pub async fn upsert_jurisdiction(&self, path: &str, name: &str, kind: &str) -> Result<i64> {
        let row = sqlx::query!(
            r#"
            INSERT INTO jurisdictions (path, name, kind)
            VALUES (?, ?, ?)
            ON CONFLICT(path) DO UPDATE SET
                name = excluded.name,
                kind = excluded.kind
            RETURNING id
            "#,
            path,
            name,
            kind
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.id)
    }

    /// Insert or get election ID
    pub async fn upsert_election(
        &self,
        jurisdiction_id: i64,
        path: &str,
        name: &str,
        date: &str,
        data_format: &str,
    ) -> Result<i64> {
        let row = sqlx::query!(
            r#"
            INSERT INTO elections (jurisdiction_id, path, name, date, data_format)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(jurisdiction_id, path) DO UPDATE SET
                name = excluded.name,
                date = excluded.date,
                data_format = excluded.data_format
            RETURNING id
            "#,
            jurisdiction_id,
            path,
            name,
            date,
            data_format
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.id)
    }

    /// Insert contest
    pub async fn insert_contest(
        &self,
        election_id: i64,
        office_id: &str,
        office_name: &str,
        jurisdiction_name: Option<&str>,
        jurisdiction_code: Option<&str>,
    ) -> Result<i64> {
        let row = sqlx::query!(
            r#"
            INSERT INTO contests (election_id, office_id, office_name, jurisdiction_name, jurisdiction_code)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(election_id, office_id) DO UPDATE SET
                office_name = excluded.office_name,
                jurisdiction_name = excluded.jurisdiction_name,
                jurisdiction_code = excluded.jurisdiction_code
            RETURNING id
            "#,
            election_id, office_id, office_name, jurisdiction_name, jurisdiction_code
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.id)
    }

    /// Get all contests for performance metrics
    pub async fn get_contests_for_election(&self, election_id: i64) -> Result<Vec<ContestInfo>> {
        let contests = sqlx::query_as!(
            ContestInfo,
            r#"
            SELECT id as "id!", election_id, office_id as office, office_name, jurisdiction_name, jurisdiction_code
            FROM contests
            WHERE election_id = ?
            ORDER BY office_id
            "#,
            election_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(contests)
    }

    /// Get all elections in the database
    pub async fn get_all_elections(&self) -> Result<Vec<ElectionInfo>> {
        let elections = sqlx::query_as!(
            ElectionInfo,
            r#"
            SELECT 
                e.id,
                e.jurisdiction_id,
                j.path as jurisdiction_path,
                j.name as jurisdiction_name,
                e.path as election_path,
                e.name,
                e.date,
                e.data_format
            FROM elections e
            JOIN jurisdictions j ON e.jurisdiction_id = j.id
            ORDER BY e.date DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(elections)
    }

    /// Get election by ID
    pub async fn get_election_by_id(&self, election_id: i64) -> Result<ElectionInfo> {
        let election = sqlx::query_as!(
            ElectionInfo,
            r#"
            SELECT 
                e.id,
                e.jurisdiction_id,
                j.path as jurisdiction_path,
                j.name as jurisdiction_name,
                e.path as election_path,
                e.name,
                e.date,
                e.data_format
            FROM elections e
            JOIN jurisdictions j ON e.jurisdiction_id = j.id
            WHERE e.id = ?
            "#,
            election_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(election)
    }

    /// Get jurisdiction by ID
    pub async fn get_jurisdiction_by_id(&self, jurisdiction_id: i64) -> Result<JurisdictionInfo> {
        let jurisdiction = sqlx::query_as!(
            JurisdictionInfo,
            r#"
            SELECT id, path, name, kind
            FROM jurisdictions
            WHERE id = ?
            "#,
            jurisdiction_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(jurisdiction)
    }

    /// Get candidates for a contest
    pub async fn get_candidates_for_contest(&self, contest_id: i64) -> Result<Vec<CandidateInfo>> {
        let candidates = sqlx::query_as!(
            CandidateInfo,
            r#"
            SELECT id as "id!", contest_id as "contest_id!", external_id, name, candidate_type as "candidate_type: CandidateType"
            FROM candidates
            WHERE contest_id = ?
            ORDER BY name
            "#,
            contest_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(candidates)
    }

    /// Get ballots for a contest
    pub async fn get_ballots_for_contest(&self, contest_id: i64) -> Result<Vec<BallotInfo>> {
        let ballots = sqlx::query_as!(
            BallotInfo,
            r#"
            SELECT id as "id!", contest_id as "contest_id!", ballot_id, precinct_id
            FROM ballots
            WHERE contest_id = ?
            ORDER BY ballot_id
            "#,
            contest_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(ballots)
    }

    /// Get choices for a ballot
    pub async fn get_choices_for_ballot(&self, ballot_id: i64) -> Result<Vec<ChoiceInfo>> {
        let choices = sqlx::query_as!(
            ChoiceInfo,
            r#"
            SELECT 
                id as "id!", 
                ballot_id as "ballot_id!", 
                candidate_id, 
                rank_position as "rank_position!", 
                choice_type
            FROM ballot_choices
            WHERE ballot_id = ?
            ORDER BY rank_position
            "#,
            ballot_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(choices)
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct ElectionInfo {
    pub id: i64,
    pub jurisdiction_id: i64,
    pub jurisdiction_path: String,
    pub jurisdiction_name: String,
    pub election_path: String,
    pub name: String,
    pub date: chrono::NaiveDate,
    pub data_format: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct JurisdictionInfo {
    pub id: i64,
    pub path: String,
    pub name: String,
    pub kind: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct CandidateInfo {
    pub id: i64,
    pub contest_id: i64,
    pub external_id: Option<String>,
    pub name: String,
    pub candidate_type: CandidateType,
}

#[derive(Debug, sqlx::FromRow)]
pub struct BallotInfo {
    pub id: i64,
    pub contest_id: i64,
    pub ballot_id: String,
    pub precinct_id: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct ChoiceInfo {
    pub id: i64,
    pub ballot_id: i64,
    pub candidate_id: Option<i64>,
    pub rank_position: i64,
    pub choice_type: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct ContestInfo {
    pub id: i64,
    pub election_id: i64,
    pub office: String,
    pub office_name: String,
    pub jurisdiction_name: Option<String>,
    pub jurisdiction_code: Option<String>,
}
