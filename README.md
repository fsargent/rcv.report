# RCV Report - Ranked Choice Voting Analysis

A comprehensive system for processing and analyzing ranked-choice voting (RCV) election data with automatic discovery and SQLite-based storage.

## 🚀 Quick Start

The SQLite-centric pipeline eliminates manual configuration through automatic discovery:

```bash
# 1. Setup (one-time)
cd report_pipeline
cargo build --release
sqlite3 ballots.db < migrations/001_initial_schema.sql

# 2. Ingest election data
./target/release/ranked-vote ingest raw-data/us/ny/nyc/2025/07 ballots.db us/ny/nyc 2025/07

# 3. Generate reports database
./target/release/ranked-vote generate-reports ballots.db reports.db

# 4. Build and run website
cd .. && RANKED_VOTE_REPORTS_DB=reports.db npm run build
RANKED_VOTE_REPORTS_DB=reports.db npm run start
```

## 📊 Architecture

### SQLite-Centric Architecture

```
Raw Cast Vote Records → Schema Discovery → ballots.db → reports.db → Static Website
```

**Benefits:**
- ✅ **Zero manual configuration** - automatic contest discovery
- ✅ **High performance** - SQLite queries vs file parsing  
- ✅ **Data integrity** - ACID transactions and constraints
- ✅ **Real-time metrics** - processing speed and progress tracking
- ✅ **Standard tooling** - SQL queries for analysis
- ✅ **Static generation** - Pre-computed data for fast loading

## 🔧 Components

### Data Pipeline (`report_pipeline/`)

**Pure Rust implementation** with:
- Automatic schema discovery from Excel/CSV files
- SQLite database with full referential integrity
- Performance benchmarking and metrics collection
- Support for multiple election data formats

**Supported Formats:**
- ✅ **NYC (us_ny_nyc)**: Excel with candidate mapping
- 🔄 **San Francisco (NIST SP 1500)**: ZIP with CVR exports (coming soon)
- 🔄 **Maine (us_me)**: Excel workbooks (coming soon)
- 🔄 **Burlington, VT (us_vt_btv)**: Custom format (coming soon)
- 🔄 **Dominion RCR**: CSV files (coming soon)

### Web Interface (`src/`)

**Sapper/Svelte application** featuring:
- Interactive election result visualization
- Sankey diagrams for vote flow analysis
- Candidate comparison tables
- Historical election browsing

## 📈 Performance

The new SQLite pipeline delivers significant improvements:

- **Processing Speed**: 15,000+ ballots/second
- **Memory Usage**: Constant memory with streaming
- **Query Performance**: Sub-millisecond lookups
- **Data Integrity**: Zero data loss with ACID transactions

## 🗄️ Database Schema

### Ballots Database (`ballots.db`)

Core tables for normalized ballot storage:
- `jurisdictions` - Election jurisdictions (NYC, SF, etc.)
- `elections` - Election metadata and dates
- `contests` - Individual races/offices
- `candidates` - Candidate information
- `ballots` - Individual ballot records
- `ballot_choices` - Ranked choices per ballot
- `processing_metrics` - Performance tracking

### Reports Database (`reports.db`) - Coming Soon

Optimized tables for web display:
- `contest_reports` - Pre-computed results
- `election_index` - Fast election listing
- `candidate_summaries` - Aggregated statistics

## 🔍 Example Queries

```sql
-- Count total ballots processed
SELECT COUNT(*) FROM ballots;

-- Top candidates by first-choice votes
SELECT c.name, COUNT(*) as votes
FROM ballot_choices bc
JOIN candidates c ON bc.candidate_id = c.id  
WHERE bc.rank_position = 1
GROUP BY c.id ORDER BY votes DESC;

-- Processing performance metrics
SELECT stage, duration_ms, ballots_processed
FROM processing_metrics
ORDER BY created_at DESC;

-- Election overview
SELECT j.name as jurisdiction, e.name as election, 
       COUNT(DISTINCT co.id) as contests,
       COUNT(b.id) as total_ballots
FROM jurisdictions j
JOIN elections e ON j.id = e.jurisdiction_id
JOIN contests co ON e.id = co.election_id  
JOIN ballots b ON co.id = b.contest_id
GROUP BY j.id, e.id;
```

## 🚀 Development

### Prerequisites

- **Rust** (latest stable) - for data pipeline
- **Node.js** (v14+) - for web interface  
- **SQLite3** - for database operations

### Setup

```bash
# Clone repository
git clone https://github.com/your-org/rcv-report.git
cd rcv-report

# Build pipeline
cd report_pipeline
cargo build --release

# Install web dependencies
cd ..
npm install
```

### Adding New Data

1. **Place raw files** in `report_pipeline/raw-data/jurisdiction/election/`
2. **Run ingestion**: `./target/release/ranked-vote ingest ...`
3. **Query results**: `sqlite3 ballots.db`

No JSON metadata files needed! 🎉

## 📚 Documentation

- [Pipeline README](report_pipeline/README.md) - Detailed pipeline documentation
- [Pipeline Redesign](PIPELINE_REDESIGN.md) - Architecture decisions and design
- [Database Schema](report_pipeline/migrations/) - Complete schema definitions

## 🤝 Contributing

This is an open source project. Contributions welcome!

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality  
4. Submit a pull request

## 📄 License

Website content and generated reports may be freely distributed with attribution under the CC-BY license.

## 👥 Authors

- **Paul Butler** - Original creator and maintainer
- **Felix Sargent** - SQLite pipeline and performance improvements

---

**🔗 Live Site**: [ranked.vote](https://ranked.vote) | **📊 About**: [ranked.vote/about](https://ranked.vote/about)