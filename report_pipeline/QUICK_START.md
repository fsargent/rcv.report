# Quick Start Guide - SQLite Pipeline

## 🚀 5-Minute Setup

### 1. Build & Setup
```bash
cd report_pipeline
cargo build --release
sqlite3 ballots.db < migrations/001_initial_schema.sql
```

### 2. Ingest Data
```bash
./target/release/ranked-vote ingest raw-data/us/ny/nyc/2025/07 ballots.db us/ny/nyc 2025/07
```

### 3. Query Results
```bash
sqlite3 ballots.db
```

## 📊 Essential Queries

```sql
-- Total ballots processed
SELECT COUNT(*) FROM ballots;

-- Contests overview
SELECT office_name, COUNT(*) as ballots 
FROM contests c JOIN ballots b ON c.id = b.contest_id 
GROUP BY c.id;

-- Top candidates (first choice)
SELECT cand.name, COUNT(*) as votes
FROM ballot_choices bc
JOIN candidates cand ON bc.candidate_id = cand.id
WHERE bc.rank_position = 1 AND bc.choice_type = 'candidate'
GROUP BY cand.id ORDER BY votes DESC LIMIT 10;

-- Processing performance
SELECT stage, duration_ms, ballots_processed,
       ROUND(ballots_processed * 1000.0 / duration_ms, 2) as rate
FROM processing_metrics 
WHERE ballots_processed > 0
ORDER BY created_at DESC;
```

## 🔧 Commands

### Available Commands
```bash
# Ingest election data
./target/release/ranked-vote ingest <raw_data_path> <db_path> <jurisdiction> <election>

# Legacy discovery (JSON output)
./target/release/ranked-vote discover <raw_data_path> <meta_dir> <jurisdiction> <election>

# Legacy report generation
./target/release/ranked-vote report <meta_dir> <raw_dir> <preprocessed_dir> <report_dir>
```

### Examples
```bash
# NYC 2025 Primary
./target/release/ranked-vote ingest raw-data/us/ny/nyc/2025/07 nyc_2025.db us/ny/nyc 2025/07

# Custom database location
./target/release/ranked-vote ingest raw-data/us/ny/nyc/2025/07 /tmp/ballots.db us/ny/nyc 2025/07
```

## 🐛 Troubleshooting

### Common Errors

**"unable to open database file"**
```bash
# Create database first
sqlite3 ballots.db < migrations/001_initial_schema.sql
```

**"UNIQUE constraint failed"**
- Expected when re-running ingestion
- Database prevents duplicate imports
- Delete database to start fresh

**"Raw data path does not exist"**
```bash
# Check path structure
ls -la raw-data/us/ny/nyc/2025/07/
```

### Performance Tips

```bash
# For large datasets, optimize SQLite
sqlite3 ballots.db "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;"

# Monitor processing speed
./target/release/ranked-vote ingest ... | grep "ballots/sec"
```

## 📈 What You Get

✅ **Automatic discovery** - No JSON metadata needed  
✅ **Performance metrics** - Real-time processing stats  
✅ **Data integrity** - ACID transactions and constraints  
✅ **Standard SQL** - Query with any SQLite tool  
✅ **Fast processing** - 15,000+ ballots/second  

## 🔗 Next Steps

- [Full Documentation](README.md)
- [Architecture Design](../PIPELINE_REDESIGN.md)
- [Database Schema](migrations/001_initial_schema.sql)
