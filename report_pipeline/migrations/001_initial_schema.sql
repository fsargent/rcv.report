-- Initial database schema for RCV ballot ingestion
-- This creates the core tables for storing ballot data

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
    precinct_id TEXT,                    -- Precinct identifier (optional)
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

-- Performance metrics table
CREATE TABLE processing_metrics (
    id INTEGER PRIMARY KEY,
    jurisdiction_path TEXT NOT NULL,
    election_path TEXT NOT NULL,
    contest_office TEXT,
    stage TEXT NOT NULL,                 -- "discovery", "file_reading", "database_insertion", etc.
    duration_ms INTEGER NOT NULL,
    ballots_processed INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for performance
CREATE INDEX idx_elections_jurisdiction ON elections(jurisdiction_id);
CREATE INDEX idx_contests_election ON contests(election_id);
CREATE INDEX idx_candidates_contest ON candidates(contest_id);
CREATE INDEX idx_ballots_contest ON ballots(contest_id);
CREATE INDEX idx_ballot_choices_ballot ON ballot_choices(ballot_id);
CREATE INDEX idx_ballot_choices_candidate ON ballot_choices(candidate_id);
CREATE INDEX idx_raw_files_election ON raw_files(election_id);
CREATE INDEX idx_raw_files_hash ON raw_files(file_hash);
CREATE INDEX idx_processing_metrics_election ON processing_metrics(jurisdiction_path, election_path);
