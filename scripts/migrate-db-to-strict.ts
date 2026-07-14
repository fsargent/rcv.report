/**
 * Rebuild the report database using SQLite STRICT tables while preserving data
 * and primary keys. Run once for an existing generated database; new databases
 * are already STRICT through init-database.ts.
 */

import { Database } from "bun:sqlite";

const dbPath = process.argv[2] || "report_pipeline/reports.sqlite3";
const db = new Database(dbPath);

const tables = ["reports", "candidates", "rounds", "allocations", "transfers"];

const existing = db
  .query("SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'reports'")
  .get() as { sql: string } | null;

if (!existing) {
  throw new Error(`No reports table found in ${dbPath}`);
}

if (/\)\s+STRICT\s*$/i.test(existing.sql)) {
  console.log(`Database already uses STRICT tables: ${dbPath}`);
  db.close();
  process.exit(0);
}

db.exec("PRAGMA foreign_keys = OFF");

db.transaction(() => {
  for (const table of tables) {
    db.exec(`ALTER TABLE ${table} RENAME TO _${table}`);
  }

  db.exec(`
    CREATE TABLE reports (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      name TEXT NOT NULL, date TEXT NOT NULL, jurisdictionPath TEXT NOT NULL,
      electionPath TEXT NOT NULL, office TEXT NOT NULL, officeName TEXT NOT NULL,
      jurisdictionName TEXT NOT NULL, electionName TEXT NOT NULL, website TEXT,
      ballotCount INTEGER NOT NULL DEFAULT 0, path TEXT NOT NULL, dataFormat TEXT,
      numCandidates INTEGER NOT NULL DEFAULT 0, winner_candidate_index INTEGER,
      condorcet INTEGER, interesting INTEGER DEFAULT 0,
      winnerNotFirstRoundLeader INTEGER DEFAULT 0, hasWriteInByName INTEGER DEFAULT 0,
      smithSet TEXT, pairwisePreferences TEXT, firstAlternate TEXT, firstFinal TEXT,
      rankingDistribution TEXT, UNIQUE(path, office)
    ) STRICT;

    CREATE TABLE candidates (
      id INTEGER PRIMARY KEY AUTOINCREMENT, report_id INTEGER NOT NULL,
      candidate_index INTEGER NOT NULL, name TEXT NOT NULL, writeIn INTEGER DEFAULT 0,
      candidate_type TEXT, firstRoundVotes INTEGER DEFAULT 0, transferVotes INTEGER DEFAULT 0,
      roundEliminated INTEGER, winner INTEGER DEFAULT 0,
      FOREIGN KEY (report_id) REFERENCES reports(id) ON DELETE CASCADE
    ) STRICT;

    CREATE TABLE rounds (
      id INTEGER PRIMARY KEY AUTOINCREMENT, report_id INTEGER NOT NULL,
      round_number INTEGER NOT NULL, undervote INTEGER DEFAULT 0, overvote INTEGER DEFAULT 0,
      continuingBallots INTEGER DEFAULT 0,
      FOREIGN KEY (report_id) REFERENCES reports(id) ON DELETE CASCADE
    ) STRICT;

    CREATE TABLE allocations (
      id INTEGER PRIMARY KEY AUTOINCREMENT, round_id INTEGER NOT NULL,
      allocatee TEXT NOT NULL, votes INTEGER NOT NULL,
      FOREIGN KEY (round_id) REFERENCES rounds(id) ON DELETE CASCADE
    ) STRICT;

    CREATE TABLE transfers (
      id INTEGER PRIMARY KEY AUTOINCREMENT, round_id INTEGER NOT NULL,
      from_candidate INTEGER NOT NULL, to_allocatee TEXT NOT NULL, count INTEGER NOT NULL,
      FOREIGN KEY (round_id) REFERENCES rounds(id) ON DELETE CASCADE
    ) STRICT;

    INSERT INTO reports SELECT * FROM _reports;
    INSERT INTO candidates SELECT * FROM _candidates;
    INSERT INTO rounds SELECT * FROM _rounds;
    INSERT INTO allocations SELECT * FROM _allocations;
    INSERT INTO transfers SELECT * FROM _transfers;

    DROP TABLE _transfers;
    DROP TABLE _allocations;
    DROP TABLE _rounds;
    DROP TABLE _candidates;
    DROP TABLE _reports;

    CREATE INDEX idx_reports_path ON reports(path);
    CREATE INDEX idx_reports_date ON reports(date);
    CREATE INDEX idx_candidates_report ON candidates(report_id);
    CREATE INDEX idx_rounds_report ON rounds(report_id);
    CREATE INDEX idx_allocations_round ON allocations(round_id);
    CREATE INDEX idx_transfers_round ON transfers(round_id);
  `);
})();

db.exec("PRAGMA foreign_keys = ON");
const violations = db.query("PRAGMA foreign_key_check").all();

if (violations.length > 0) {
  db.close();
  throw new Error(`Foreign-key validation failed: ${JSON.stringify(violations)}`);
}

db.exec("VACUUM");
db.close();

console.log(`Migrated database to STRICT tables: ${dbPath}`);
