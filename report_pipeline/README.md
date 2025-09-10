# ranked.vote

A system for processing and analyzing ranked-choice voting (RCV) election data. This repository contains:

- Data processing pipeline for converting raw ballot data into standardized formats
- Report generation for detailed election analysis
- Web interface for viewing election results and analysis

## Project Structure

- `election-metadata/` - Election configuration files (git submodule)
- `reports/` - Generated election reports (git submodule)
- `raw-data/` - Raw ballot data (downloaded during setup)
- `preprocessed/` - Processed ballot data (generated)

## Setup

1. Install dependencies:

   - Rust (latest stable)
   - Node.js (v10 or later)
   - AWS CLI (configured with appropriate credentials)

2. Clone this repository with submodules:

```bash
git clone --recursive git@github.com:ranked-vote/ranked-vote.git
cd ranked-vote
```

Or if you've already cloned the repository:

```bash
git submodule init
git submodule update
```

3. Download data:

```bash
./mount.sh
```

This will:

- Initialize and update the submodules (`election-metadata` and `reports`)
- Download raw ballot data from S3

## Usage

### SQLite-Based Pipeline

The pipeline eliminates manual configuration through automatic discovery:

#### 1. **Setup** (One-time)

```bash
# Build the pipeline
cargo build --release

# Create database with schema
sqlite3 ballots.db < migrations/001_initial_schema.sql
```

#### 2. **Ingest Election Data**

```bash
# Automatically discover contests and import to SQLite database
./target/release/ranked-vote ingest raw-data/us/ny/nyc/2025/07 ballots.db us/ny/nyc 2025/07
```

**What this does:**
- ✅ **Auto-discovers contests** from Excel files (no JSON metadata needed!)
- ✅ **Imports all ballot data** with full referential integrity
- ✅ **Tracks performance metrics** (ballots/second, processing time)
- ✅ **Provides real-time progress** with colored output
- ✅ **Enforces data constraints** (prevents duplicate ballots)

**Example output:**
```
🚀 Starting SQLite ingestion for us/ny/nyc 2025/07
📋 Discovered 1 contests
✅ Database initialized: ballots.db
🚀 Starting ingestion for us/ny/nyc 2025/07
  📊 Processing contest: DEM Borough President - Manhattan
    ✅ Processed 15,847 ballots for DEM Borough President - Manhattan
🎉 Ingestion completed! Processed 15,847 ballots in 2.34 seconds
```

#### 3. **Query and Analyze Data**

```bash
# Connect to database
sqlite3 ballots.db

# Example queries
.mode column
.headers on

-- Count total ballots
SELECT COUNT(*) as total_ballots FROM ballots;

-- Ballots per contest
SELECT c.office_name, COUNT(*) as ballot_count 
FROM contests c JOIN ballots b ON c.id = b.contest_id 
GROUP BY c.id;

-- Performance metrics
SELECT stage, duration_ms, ballots_processed 
FROM processing_metrics 
ORDER BY created_at DESC;

-- Top candidates by first-choice votes
SELECT cand.name, COUNT(*) as first_choice_votes
FROM ballot_choices bc
JOIN candidates cand ON bc.candidate_id = cand.id
WHERE bc.rank_position = 1 AND bc.choice_type = 'candidate'
GROUP BY cand.id
ORDER BY first_choice_votes DESC;
```

#### 4. **Supported Data Formats**

Currently implemented:
- **NYC (us_ny_nyc)**: Excel files with candidate mapping ✅

Coming soon:
- San Francisco (NIST SP 1500)
- Maine (us_me)
- Burlington, VT (us_vt_btv)
- Dominion RCR
- Simple JSON

## Adding Election Data

### 1. Prepare Election Metadata

Create or modify the jurisdiction metadata file in `election-metadata/` following this structure:

- US jurisdictions: `us/{state}/{city}.json` (e.g., `us/ca/sfo.json`)
- Other locations: `{country}/{region}/{city}.json`

The metadata file must specify:

- Data format (supported formats: `nist_sp_1500`, `us_me`, `us_vt_btv`, `dominion_rcr`, `us_ny_nyc`, `simple_json`)
- Election date
- Offices and contests
- Loader parameters specific to the format

### 2. Prepare Raw Data

1. Create the corresponding directory structure in `raw-data/` matching your metadata path
2. Add your raw ballot data files in the correct format:
   - San Francisco (NIST SP 1500): ZIP containing CVR exports
   - Maine: Excel workbooks
   - NYC: Excel workbooks with candidate mapping
   - Dominion RCR: CSV files
   - Simple JSON: JSON files following the schema

Example structure:

```text
raw-data/
└── us/
    └── ca/
        └── sfo/
            └── 2023/
                └── 11/
                    ├── mayor/
                    │   └── cvr.zip
                    └── supervisor/
                        └── cvr.zip
```

### 3. Process and Verify

1. Run `./sync.sh` to:

   - Verify directory structure
   - Generate file hashes
   - Update metadata

2. Run `./report.sh` to:

   - Convert raw data to normalized format
   - Generate analysis reports
   - Verify data integrity

3. Check generated files:
   - Preprocessed data: `preprocessed/{jurisdiction_path}/normalized.json.gz`
   - Reports: `reports/{jurisdiction_path}/report.json`

### 4. Submit Changes

1. Commit your changes in both submodules:

   ```bash
   cd election-metadata
   git add .
   git commit -m "Add {jurisdiction} {date} election"

   cd ../reports
   git add .
   git commit -m "Add {jurisdiction} {date} reports"
   ```

2. Push to your fork and open pull requests for both repositories:
   - ranked-vote/election-metadata
   - ranked-vote/reports

### Supported Data Formats

For format-specific requirements and examples, see the documentation for each supported format:

- `nist_sp_1500`: San Francisco format following NIST SP 1500-103 standard
- `us_me`: Maine state format (Excel-based)
- `us_vt_btv`: Burlington, VT format
- `dominion_rcr`: Dominion RCV format
- `us_ny_nyc`: NYC Board of Elections format
- `simple_json`: Simple JSON format for testing and small elections

## Data Flow

1. Raw ballot data (various formats) → `raw-data/`
2. Processing pipeline converts to standardized format → `preprocessed/`
3. Report generation creates detailed analysis → `reports/`
4. Web interface displays results

## Supported Election Formats

- San Francisco (NIST SP 1500)
- Maine
- Burlington, VT
- Dominion RCR
- NYC
- Simple JSON

## Troubleshooting

### Common Issues

**Database creation fails:**
```bash
# Ensure SQLite is installed
sqlite3 --version

# Create database manually
sqlite3 ballots.db < migrations/001_initial_schema.sql
```

**Ingestion fails with "file not found":**
```bash
# Check file structure
ls -la raw-data/us/ny/nyc/2025/07/

# Ensure you have the candidate mapping file
ls -la raw-data/us/ny/nyc/2025/07/*CandidacyID_To_Name.xlsx
```

**UNIQUE constraint errors:**
- This is expected behavior when re-running ingestion
- The database prevents duplicate ballot imports
- Delete and recreate database to start fresh

### Performance Analysis

```sql
-- Connect to database
sqlite3 ballots.db

-- View processing metrics
SELECT 
  stage,
  duration_ms,
  ballots_processed,
  ROUND(ballots_processed * 1000.0 / duration_ms, 2) as ballots_per_second
FROM processing_metrics 
WHERE ballots_processed IS NOT NULL
ORDER BY created_at;

-- Database statistics
SELECT 
  'Total Ballots' as metric, COUNT(*) as count FROM ballots
UNION ALL
SELECT 'Total Candidates', COUNT(*) FROM candidates  
UNION ALL
SELECT 'Total Contests', COUNT(*) FROM contests;
```

### Advanced Usage

**Custom database location:**
```bash
./target/release/ranked-vote ingest raw-data/path /custom/path/ballots.db jurisdiction election
```

**Performance tuning for large datasets:**
```bash
# Optimize SQLite settings
sqlite3 ballots.db "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;"
```

## License

Website content and generated reports may be freely distributed with attribution under the CC-BY license.

## Contributing

This is an open source project. For more information about contributing, please see the [about page](https://ranked.vote/about).

## Author

Created and maintained by [Paul Butler](https://paulbutler.org).
