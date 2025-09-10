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

### Processing Election Data

1. Download the raw ballot data from s3

```bash
./mount.sh
```

2. Sync raw data with metadata:

```bash
./sync.sh
```

3. Generate reports:

```bash
./report.sh
```

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

## License

Website content and generated reports may be freely distributed with attribution under the CC-BY license.

## Contributing

This is an open source project. For more information about contributing, please see the [about page](https://ranked.vote/about).

## Author

Created and maintained by [Paul Butler](https://paulbutler.org).
