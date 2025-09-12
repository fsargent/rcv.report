# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Project Overview

RCV Report is a comprehensive system for processing and analyzing ranked-choice voting (RCV) election data. It consists of a dual architecture:

1. **Data Pipeline** (`report_pipeline/`): Rust-based SQLite-centric pipeline for ballot processing
2. **Web Interface** (`src/`): SvelteKit application for visualization and analysis

The project is transitioning from a file-based architecture to a SQLite-centric approach for better performance and reduced manual configuration.

## Essential Commands

### Pipeline Development (Rust)

```bash
# Build the data pipeline
cd report_pipeline
cargo build --release

# Run tests
cargo test

# Create database schema
sqlite3 ballots.db < migrations/001_initial_schema.sql

# Ingest election data (NYC format example)
./target/release/ranked-vote ingest raw-data/us/ny/nyc/2025/07 ballots.db us/ny/nyc 2025/07

# Generate reports database  
./target/release/ranked-vote generate-reports ballots.db reports.db
```

### Web Development (SvelteKit)

```bash
# Install dependencies
npm install

# Development server
npm run dev
# OR use the provided script with environment variable
RANKED_VOTE_REPORTS_DB=reports.db npm run dev

# Build for production  
npm run build

# Start production server
npm run start

# Run tests
npm run test
```

### Database Operations

```bash
# Connect to ballots database
sqlite3 ballots.db

# Connect to reports database  
sqlite3 reports.db

# View processing performance
sqlite3 ballots.db "SELECT stage, duration_ms, ballots_processed FROM processing_metrics ORDER BY created_at DESC;"

# Count total ballots
sqlite3 ballots.db "SELECT COUNT(*) FROM ballots;"
```

## Architecture

### Core Data Flow

```
Raw Cast Vote Records → Schema Discovery → ballots.db → reports.db → SvelteKit Website
```

### Key Components

**Data Pipeline (`report_pipeline/`)**:
- **SQLite Storage**: `ballots.db` for normalized ballot data, `reports.db` for pre-computed results
- **Schema Discovery**: Automatic contest detection from raw files (eliminating manual JSON metadata)
- **Format Support**: Currently NYC Excel format, expanding to NIST SP 1500, Maine, Vermont, Dominion RCR
- **Performance Tracking**: Real-time metrics collection and benchmarking

**Web Interface (`src/`)**:
- **SvelteKit Framework**: Modern static site generation with dynamic capabilities
- **Database Integration**: Direct SQLite access via `better-sqlite3`
- **Visualization**: Sankey diagrams, candidate comparison tables, interactive results
- **Static Generation**: Pre-computed pages for fast loading

### Database Schema

**Ballots Database** (`ballots.db`):
- `jurisdictions` - Election jurisdictions (NYC, SF, etc.)
- `elections` - Election metadata and dates  
- `contests` - Individual races/offices
- `candidates` - Candidate information
- `ballots` - Individual ballot records
- `ballot_choices` - Ranked choices per ballot
- `processing_metrics` - Performance tracking

**Reports Database** (`reports.db`):
- `contest_reports` - Pre-computed election results
- `election_index` - Fast election listing
- `candidate_summaries` - Aggregated statistics

### File Structure

```
├── report_pipeline/           # Rust data processing pipeline
│   ├── src/
│   │   ├── commands/         # CLI command implementations
│   │   ├── database/         # SQLite schema and operations
│   │   ├── formats/          # Election data format parsers
│   │   └── model/           # Data models and types
│   ├── migrations/          # Database schema migrations
│   └── raw-data/           # Raw election data files
├── src/                    # SvelteKit web application
│   ├── components/         # Reusable Svelte components
│   ├── routes/            # Page routes and API endpoints
│   ├── lib/               # Utility functions and types
│   └── reports_sqlite.ts  # SQLite database integration
└── static/               # Static web assets
```

## Development Patterns

### Adding New Election Data Formats

1. Create parser in `report_pipeline/src/formats/`
2. Implement `DataLoader` trait for the new format
3. Add format detection logic in schema discovery
4. Update CLI to support the new format identifier

### Database Schema Changes

1. Create new migration file in `report_pipeline/migrations/`
2. Update model definitions in `report_pipeline/src/model/`
3. Modify database operations in `report_pipeline/src/database/`
4. Test migration with existing data

### Web Interface Development

1. Database queries go in `src/reports_sqlite.ts`
2. Page routes are file-based in `src/routes/`
3. Shared components in `src/components/`
4. Use TypeScript interfaces from `src/report_types.ts`

## Environment Variables

- `RANKED_VOTE_REPORTS_DB`: Path to reports database file (default: `reports.db`)
- `NODE_ENV`: Set to `production` for optimized builds

## Common Development Tasks

### Running a Single Test

```bash
# Rust tests
cd report_pipeline
cargo test test_name

# Web tests  
npm test -- --grep "test description"
```

### Database Debugging

```bash
# View database schema
sqlite3 ballots.db ".schema"

# Export data for inspection
sqlite3 ballots.db ".output ballots.csv" ".mode csv" "SELECT * FROM ballots LIMIT 100;"

# Performance analysis
sqlite3 ballots.db "EXPLAIN QUERY PLAN SELECT * FROM ballots JOIN ballot_choices ON ballots.id = ballot_choices.ballot_id;"
```

### Performance Optimization

The pipeline includes built-in performance tracking. View metrics with:

```sql
SELECT 
  stage,
  duration_ms,
  ballots_processed,
  ROUND(ballots_processed * 1000.0 / duration_ms, 2) as ballots_per_second
FROM processing_metrics 
ORDER BY created_at DESC;
```

## Key Dependencies

**Rust Pipeline**:
- `sqlx` - Async SQLite operations
- `calamine` - Excel file parsing  
- `serde_json` - JSON serialization
- `tokio` - Async runtime

**Web Interface**:
- `@sveltejs/kit` - SvelteKit framework
- `better-sqlite3` - SQLite database access
- `tippy.js` - Tooltips and popovers
- `vite` - Build tool and dev server

## Migration Notes

The project is migrating from a file-based system with JSON metadata to SQLite-centric architecture. Key changes:

- Raw data processing now uses automatic schema discovery instead of manual JSON configuration
- Reports are generated into SQLite database instead of static JSON files  
- Web interface queries SQLite directly instead of reading static files
- Git submodules for `election-metadata` and `reports` are being phased out
