# RCV Pipeline Redesign: SQLite-Centric Architecture

## Executive Summary

This document outlines a comprehensive redesign of the RCV (Ranked Choice Voting) pipeline to streamline data processing, reduce manual configuration, and leverage SQLite as the core data storage and processing engine. The new architecture will move from a file-based system with extensive JSON metadata to a database-driven approach with automatic schema discovery.

## Current System Analysis

### Current Architecture
```
Raw Data (XLSX/CSV/JSON) → JSON Metadata → Rust Pipeline → JSON Reports → Sapper Website
                         ↑
                   Manual Configuration
```

### Current Pain Points

1. **Heavy Manual Configuration**: Each election requires extensive JSON metadata files with:
   - Office definitions
   - Contest mappings
   - Loader parameters
   - File hash management

2. **File-Based Processing**: Data flows through multiple file formats:
   - Raw data in various formats (XLSX, CSV, JSON)
   - Preprocessed data as compressed JSON
   - Reports as JSON files
   - Index files for navigation

3. **Limited Discoverability**: While a `discover` command exists for NYC data, it's format-specific and still requires manual metadata creation

4. **Fragmented Data Access**: Website reads from static JSON files, making dynamic queries difficult

5. **Complex Deployment**: Multiple git submodules, S3 syncing, and file-based report generation

## Proposed Architecture

### New Data Flow
```
Raw Cast Vote Records → ballots.db → reports.db → Dynamic Web API
                      ↑              ↑
              Schema Discovery   Report Generation
```

### Core Components

#### 1. Ballots Database (`ballots.db`)
- **Purpose**: Normalized storage of all cast vote records
- **Schema**: Standardized across all jurisdictions
- **Benefits**: 
  - Single source of truth for ballot data
  - Enables cross-election analysis
  - Supports incremental updates
  - Allows for complex queries

#### 2. Reports Database (`reports.db`)
- **Purpose**: Pre-computed election results and analysis
- **Schema**: Optimized for web display
- **Benefits**:
  - Fast web queries
  - Cached computations
  - Historical comparisons
  - Flexible reporting

#### 3. Schema Discovery Engine
- **Purpose**: Automatically detect election structure from raw data
- **Implementation**: Enhanced version of current `discover` command
- **Benefits**:
  - Minimal manual configuration
  - Consistent data interpretation
  - Extensible to new formats

## Detailed Design

### Database Schemas

#### Ballots Database Schema

```sql
-- Jurisdictions table
CREATE TABLE jurisdictions (
    id INTEGER PRIMARY KEY,
    path TEXT UNIQUE NOT NULL,           -- e.g., "us/ny/nyc"
    name TEXT NOT NULL,                  -- e.g., "New York City"
    kind TEXT NOT NULL,                  -- e.g., "city", "state", "county"
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Elections table
CREATE TABLE elections (
    id INTEGER PRIMARY KEY,
    jurisdiction_id INTEGER NOT NULL,
    path TEXT NOT NULL,                  -- e.g., "2025/07"
    name TEXT NOT NULL,                  -- e.g., "Primary Election"
    date DATE NOT NULL,
    data_format TEXT NOT NULL,           -- e.g., "us_ny_nyc"
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (jurisdiction_id) REFERENCES jurisdictions(id),
    UNIQUE(jurisdiction_id, path)
);

-- Contests table
CREATE TABLE contests (
    id INTEGER PRIMARY KEY,
    election_id INTEGER NOT NULL,
    office_id TEXT NOT NULL,             -- e.g., "mayor", "borough-president-manhattan"
    office_name TEXT NOT NULL,           -- e.g., "Mayor", "Borough President - Manhattan"
    jurisdiction_name TEXT,              -- e.g., "Citywide", "Manhattan"
    jurisdiction_code TEXT,              -- e.g., "026918"
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (election_id) REFERENCES elections(id),
    UNIQUE(election_id, office_id)
);

-- Candidates table
CREATE TABLE candidates (
    id INTEGER PRIMARY KEY,
    contest_id INTEGER NOT NULL,
    external_id TEXT,                    -- Original ID from source data
    name TEXT NOT NULL,
    candidate_type TEXT NOT NULL,        -- "regular", "write_in"
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (contest_id) REFERENCES contests(id)
);

-- Ballots table
CREATE TABLE ballots (
    id INTEGER PRIMARY KEY,
    contest_id INTEGER NOT NULL,
    ballot_id TEXT NOT NULL,             -- Original ballot identifier
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (contest_id) REFERENCES contests(id),
    UNIQUE(contest_id, ballot_id)
);

-- Ballot choices table
CREATE TABLE ballot_choices (
    id INTEGER PRIMARY KEY,
    ballot_id INTEGER NOT NULL,
    rank_position INTEGER NOT NULL,     -- 1, 2, 3, etc.
    choice_type TEXT NOT NULL,          -- "candidate", "undervote", "overvote"
    candidate_id INTEGER,               -- NULL for undervote/overvote
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (ballot_id) REFERENCES ballots(id),
    FOREIGN KEY (candidate_id) REFERENCES candidates(id),
    UNIQUE(ballot_id, rank_position)
);

-- Raw files tracking
CREATE TABLE raw_files (
    id INTEGER PRIMARY KEY,
    election_id INTEGER NOT NULL,
    filename TEXT NOT NULL,
    file_hash TEXT NOT NULL,
    file_size INTEGER,
    processed_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (election_id) REFERENCES elections(id),
    UNIQUE(election_id, filename)
);
```

#### Reports Database Schema

```sql
-- Contest reports table
CREATE TABLE contest_reports (
    id INTEGER PRIMARY KEY,
    jurisdiction_path TEXT NOT NULL,
    election_path TEXT NOT NULL,
    contest_office TEXT NOT NULL,
    contest_name TEXT NOT NULL,
    winner_name TEXT NOT NULL,
    num_candidates INTEGER NOT NULL,
    num_rounds INTEGER NOT NULL,
    total_ballots INTEGER NOT NULL,
    report_data JSON NOT NULL,           -- Full report JSON
    generated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(jurisdiction_path, election_path, contest_office)
);

-- Election index
CREATE TABLE election_index (
    id INTEGER PRIMARY KEY,
    jurisdiction_path TEXT NOT NULL,
    election_path TEXT NOT NULL,
    jurisdiction_name TEXT NOT NULL,
    election_name TEXT NOT NULL,
    election_date DATE NOT NULL,
    contest_count INTEGER NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(jurisdiction_path, election_path)
);

-- Performance metrics
CREATE TABLE processing_metrics (
    id INTEGER PRIMARY KEY,
    jurisdiction_path TEXT NOT NULL,
    election_path TEXT NOT NULL,
    contest_office TEXT,
    stage TEXT NOT NULL,                 -- "discovery", "import", "report_generation"
    duration_ms INTEGER NOT NULL,
    ballots_processed INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### Implementation Plan

#### Phase 1: Enhanced Discovery Engine

**Goal**: Expand the current `discover` command to work with multiple formats and generate minimal metadata.

**Implementation** (Rust):
```rust
// Enhanced discovery system
pub struct DiscoveryEngine {
    format_detectors: Vec<Box<dyn FormatDetector>>,
}

pub trait FormatDetector {
    fn can_handle(&self, path: &Path) -> bool;
    fn discover_contests(&self, path: &Path) -> Result<Vec<Contest>, DiscoveryError>;
    fn get_format_name(&self) -> &str;
}

// Implementations for each format
pub struct NycFormatDetector;
pub struct SfoFormatDetector;
pub struct MaineFormatDetector;
// ... etc
```

**Benefits**:
- Automatic format detection
- Consistent contest discovery
- Extensible to new formats
- Minimal manual configuration

#### Phase 2: Ballots Database Implementation

**Goal**: Create the core ballots database and import pipeline.

**Implementation** (Pure Rust):

```rust
use sqlx::{SqlitePool, Row};
use tokio;
use calamine::{open_workbook_auto, Reader};

pub struct BallotsDatabase {
    pool: SqlitePool,
}

impl BallotsDatabase {
    pub async fn new(db_path: &str) -> Result<Self, sqlx::Error> {
        let pool = SqlitePool::connect(db_path).await?;
        
        // Run migrations to create schema
        sqlx::migrate!("./migrations").run(&pool).await?;
        
        Ok(Self { pool })
    }
    
    pub async fn import_election(&self, raw_path: &Path, discovered_contests: &[Contest]) -> Result<(), ImportError> {
        let mut tx = self.pool.begin().await?;
        
        // Insert jurisdiction, election, contests
        let jurisdiction_id = self.insert_jurisdiction(&mut tx, &discovered_contests[0].jurisdiction).await?;
        let election_id = self.insert_election(&mut tx, jurisdiction_id, &discovered_contests[0].election).await?;
        
        // Process each contest
        for contest in discovered_contests {
            let contest_id = self.insert_contest(&mut tx, election_id, contest).await?;
            
            // Use existing calamine-based readers to process ballot data
            let ballots = self.read_contest_ballots(raw_path, contest)?;
            self.insert_ballots(&mut tx, contest_id, &ballots).await?;
        }
        
        tx.commit().await?;
        Ok(())
    }
    
    fn read_contest_ballots(&self, raw_path: &Path, contest: &Contest) -> Result<Vec<Ballot>, ImportError> {
        // Leverage existing format readers (us_ny_nyc, nist_sp_1500, etc.)
        // This reuses all the proven Excel processing logic
        match contest.data_format.as_str() {
            "us_ny_nyc" => crate::formats::us_ny_nyc::read_ballots(raw_path, &contest.loader_params),
            "nist_sp_1500" => crate::formats::nist_sp_1500::read_ballots(raw_path, &contest.loader_params),
            // ... other formats
            _ => Err(ImportError::UnsupportedFormat(contest.data_format.clone()))
        }
    }
    
    pub async fn query_ballots(&self, contest_id: i64) -> Result<Vec<Ballot>, QueryError> {
        // Optimized ballot retrieval with joins
        let ballots = sqlx::query_as!(
            Ballot,
            r#"
            SELECT b.ballot_id, bc.rank_position, bc.choice_type, c.name as candidate_name
            FROM ballots b
            JOIN ballot_choices bc ON b.id = bc.ballot_id
            LEFT JOIN candidates c ON bc.candidate_id = c.id
            WHERE b.contest_id = ?
            ORDER BY b.ballot_id, bc.rank_position
            "#,
            contest_id
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(ballots)
    }
}
```

**Benefits of Pure Rust Approach**:
- Reuses existing, proven Excel processing code
- Type-safe database operations with compile-time query checking
- Single binary deployment
- Excellent performance for large datasets
- No language boundary overhead

#### Phase 3: Report Generation Pipeline

**Goal**: Generate reports from the ballots database and store in reports database.

**Key Features**:
- Incremental report generation (only update changed contests)
- Parallel processing for multiple contests
- Caching of intermediate results
- Historical comparison capabilities

**Implementation** (Pure Rust):
```rust
use crate::tabulator::RcvTabulator;
use crate::model::report::ContestReport;

pub struct ReportGenerator {
    ballots_db: BallotsDatabase,
    reports_db: ReportsDatabase,
}

impl ReportGenerator {
    pub fn new(ballots_db: BallotsDatabase, reports_db: ReportsDatabase) -> Self {
        Self { ballots_db, reports_db }
    }
    
    pub async fn generate_contest_report(&self, contest_id: i64) -> Result<ContestReport, ReportError> {
        // Load ballots from database
        let ballots = self.ballots_db.query_ballots(contest_id).await?;
        
        // Run RCV tabulation using existing tabulator
        let mut tabulator = RcvTabulator::new();
        let report = tabulator.tabulate(&ballots)?;
        
        // Store in reports database
        self.reports_db.store_report(contest_id, &report).await?;
        
        Ok(report)
    }
    
    pub async fn generate_all_reports(&self, force_regenerate: bool) -> Result<(), ReportError> {
        let contests = self.ballots_db.get_all_contests().await?;
        
        // Use rayon for parallel processing
        use rayon::prelude::*;
        
        contests.par_iter().try_for_each(|contest| {
            // Skip if report exists and not forcing regeneration
            if !force_regenerate && self.reports_db.report_exists(contest.id)? {
                return Ok(());
            }
            
            // Generate report
            tokio::runtime::Handle::current().block_on(async {
                self.generate_contest_report(contest.id).await
            })?;
            
            Ok(())
        })?;
        
        Ok(())
    }
}
```

**Benefits**:
- Reuses existing RCV tabulation algorithms
- Parallel processing with `rayon`
- Type-safe database operations
- Incremental report generation

#### Phase 4: Dynamic Web API

**Goal**: Replace static JSON files with dynamic database queries.

**New API Endpoints**:
```typescript
// Replace current static file serving
GET /api/elections                    // List all elections
GET /api/elections/{path}            // Get election details
GET /api/elections/{path}/contests   // List contests in election
GET /api/contests/{id}/report        // Get contest report
GET /api/contests/{id}/ballots       // Get raw ballot data (for analysis)

// New analytical endpoints
GET /api/jurisdictions               // List all jurisdictions
GET /api/search?q={query}           // Search across elections
GET /api/compare?contests={ids}     // Compare multiple contests
```

**Implementation Options**:

**Option A: Rust Web Server** (Recommended for consistency):
```rust
use axum::{extract::Path, response::Json, routing::get, Router};
use serde_json::Value;

pub fn create_api_router(reports_db: ReportsDatabase) -> Router {
    Router::new()
        .route("/api/elections", get(list_elections))
        .route("/api/elections/:path", get(get_election))
        .route("/api/contests/:id/report", get(get_contest_report))
        .with_state(reports_db)
}

async fn list_elections(
    State(db): State<ReportsDatabase>
) -> Result<Json<Value>, ApiError> {
    let elections = db.get_all_elections().await?;
    Ok(Json(serde_json::to_value(elections)?))
}

async fn get_contest_report(
    Path(contest_id): Path<i64>,
    State(db): State<ReportsDatabase>
) -> Result<Json<Value>, ApiError> {
    let report = db.get_contest_report(contest_id).await?;
    Ok(Json(report.report_data))
}
```

**Option B: Keep SvelteKit Frontend** (Hybrid approach):
- Rust CLI for data processing
- Node.js/SvelteKit for web serving
- SQLite database shared between both

```typescript
// src/routes/api/elections/+server.ts
import Database from 'better-sqlite3';

export async function GET({ url }) {
    const db = new Database('reports.db');
    const elections = db.prepare(`
        SELECT jurisdiction_path, election_path, jurisdiction_name, 
               election_name, election_date, contest_count
        FROM election_index 
        ORDER BY election_date DESC
    `).all();
    
    return json(elections);
}
```

### Migration Strategy

#### Step 1: Parallel Implementation
- Keep existing system running
- Implement new pipeline alongside
- Compare outputs for validation

#### Step 2: Gradual Migration
- Start with newest elections
- Migrate one jurisdiction at a time
- Maintain backward compatibility

#### Step 3: Full Cutover
- Switch web API to database backend
- Archive old JSON files
- Update deployment scripts

### Technology Recommendations

#### Language Choice: Pure Rust End-to-End

**Rationale for Pure Rust**:
- Existing codebase already has excellent Excel processing with `calamine`
- Current `discover` command provides a solid foundation to build upon
- SQLite integration is mature and performant with `sqlx`
- Single binary deployment simplifies operations
- Consistent toolchain and dependencies
- Superior performance for large datasets

**Rust Advantages**:
- **Excel Processing**: `calamine` crate already proven in current system
- **SQLite Integration**: `sqlx` provides async, type-safe database operations
- **Performance**: Native speed for data processing and tabulation
- **Memory Safety**: Eliminates entire classes of bugs
- **Deployment**: Single binary with no runtime dependencies
- **Ecosystem**: Rich crate ecosystem for all needed functionality

#### Database Choice: SQLite

**Advantages**:
- Zero configuration
- Excellent performance for read-heavy workloads
- ACID transactions
- Full-text search capabilities
- Easy backup and replication
- Works well with both Python and Rust

**Considerations**:
- Single writer limitation (mitigated by batch processing)
- File-based (good for this use case)
- Excellent tooling ecosystem

### Expected Benefits

#### For Developers
- **Reduced Configuration**: Automatic discovery eliminates most manual JSON editing
- **Better Debugging**: SQL queries for data exploration
- **Faster Development**: Standard database tools and practices
- **Easier Testing**: Database fixtures and rollbacks

#### For Users
- **Faster Website**: Database queries vs. file system access
- **Better Search**: Full-text search across all elections
- **Dynamic Analysis**: Real-time queries and comparisons
- **Mobile Performance**: Optimized API responses

#### For Operations
- **Simpler Deployment**: Fewer moving parts, single database files
- **Better Monitoring**: Database metrics and query analysis
- **Easier Backup**: Standard database backup procedures
- **Incremental Updates**: Only process changed data

### Implementation Timeline

#### Month 1: Foundation (Pure Rust)
- [ ] Add SQLite dependencies to Cargo.toml (`sqlx`, `tokio`)
- [ ] Design and implement database schemas with migrations
- [ ] Enhance existing `discover` command to output to SQLite
- [ ] Build basic import pipeline for NYC format (reusing existing code)

#### Month 2: Core Pipeline (Pure Rust)
- [ ] Extend discovery engine to all existing formats
- [ ] Implement ballots database import for all formats
- [ ] Create report generation pipeline (reusing existing tabulator)
- [ ] Build reports database population

#### Month 3: Web Integration
- [ ] Implement new CLI commands (`discover`, `report`, `serve`)
- [ ] Choose web serving approach (pure Rust vs. hybrid)
- [ ] Update frontend to use database API
- [ ] Performance testing and optimization

#### Month 4: Migration & Polish
- [ ] Migrate existing data to new system
- [ ] Comprehensive testing against legacy outputs
- [ ] Update deployment scripts for single binary
- [ ] Documentation and training

### Risk Mitigation

#### Data Integrity
- **Solution**: Comprehensive validation during import
- **Backup**: Keep original files as source of truth
- **Testing**: Automated comparison with existing outputs

#### Performance
- **Solution**: Database indexing and query optimization
- **Monitoring**: Performance metrics collection
- **Fallback**: Caching layer if needed

#### Complexity
- **Solution**: Incremental implementation and testing
- **Documentation**: Comprehensive developer documentation
- **Training**: Team knowledge sharing sessions

## Conclusion

This redesign represents a significant architectural improvement that will:

1. **Reduce Manual Work**: Automatic discovery and minimal configuration
2. **Improve Performance**: Database-optimized queries and caching
3. **Enable New Features**: Cross-election analysis and dynamic reporting
4. **Simplify Operations**: Standard database practices and tooling
5. **Future-Proof**: Extensible architecture for new data formats

The SQLite-centric approach leverages proven database technology while maintaining the simplicity and portability that makes the current system successful. The pure Rust implementation builds upon the existing codebase's strengths while providing superior performance and deployment simplicity.

The migration can be done incrementally with minimal risk, allowing for thorough testing and validation at each step.
