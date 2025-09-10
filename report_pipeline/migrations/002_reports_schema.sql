-- Reports database schema for optimized web queries
-- This database contains pre-computed results and summaries

-- Election index for the main page
CREATE TABLE election_index (
    id INTEGER PRIMARY KEY,
    path TEXT UNIQUE NOT NULL,
    jurisdiction_name TEXT NOT NULL,
    election_name TEXT NOT NULL,
    date TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Contest summaries for election index
CREATE TABLE contest_summaries (
    id INTEGER PRIMARY KEY,
    election_path TEXT NOT NULL,
    office TEXT NOT NULL,
    office_name TEXT NOT NULL,
    name TEXT NOT NULL,
    winner TEXT,
    num_candidates INTEGER NOT NULL,
    num_rounds INTEGER NOT NULL,
    ballot_count INTEGER NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (election_path) REFERENCES election_index(path)
);

-- Full contest reports (JSON blob for detailed analysis)
CREATE TABLE contest_reports (
    id INTEGER PRIMARY KEY,
    path TEXT UNIQUE NOT NULL, -- e.g., "us/ny/nyc/2025/07/council-member-8th-council-district"
    election_path TEXT NOT NULL,
    office TEXT NOT NULL,
    report_json TEXT NOT NULL, -- Full JSON report for the contest
    ballot_count INTEGER NOT NULL,
    winner TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (election_path) REFERENCES election_index(path)
);

-- Round-by-round results for detailed analysis
CREATE TABLE contest_rounds (
    id INTEGER PRIMARY KEY,
    contest_path TEXT NOT NULL,
    round_number INTEGER NOT NULL,
    candidate_name TEXT NOT NULL,
    votes INTEGER NOT NULL,
    percentage REAL,
    eliminated BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (contest_path) REFERENCES contest_reports(path),
    UNIQUE(contest_path, round_number, candidate_name)
);

-- Candidate performance across rounds
CREATE TABLE candidate_performance (
    id INTEGER PRIMARY KEY,
    contest_path TEXT NOT NULL,
    candidate_name TEXT NOT NULL,
    first_choice_votes INTEGER NOT NULL,
    final_votes INTEGER,
    elimination_round INTEGER, -- NULL if not eliminated
    vote_transfers_in INTEGER DEFAULT 0,
    vote_transfers_out INTEGER DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (contest_path) REFERENCES contest_reports(path),
    UNIQUE(contest_path, candidate_name)
);

-- Indexes for fast queries
CREATE INDEX idx_contest_summaries_election ON contest_summaries(election_path);
CREATE INDEX idx_contest_reports_election ON contest_reports(election_path);
CREATE INDEX idx_contest_rounds_contest ON contest_rounds(contest_path, round_number);
CREATE INDEX idx_candidate_performance_contest ON candidate_performance(contest_path);
CREATE INDEX idx_election_index_date ON election_index(date DESC);
