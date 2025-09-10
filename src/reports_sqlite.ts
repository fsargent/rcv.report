import * as Database from 'better-sqlite3';
import type { IReportIndex, IContestReport } from "./report_types";

const { RANKED_VOTE_REPORTS_DB } = process.env;

let db: Database.Database | null = null;

function getDatabase(): Database.Database {
    if (!db) {
        if (!RANKED_VOTE_REPORTS_DB) {
            throw new Error('RANKED_VOTE_REPORTS_DB environment variable not set');
        }
        db = new Database(RANKED_VOTE_REPORTS_DB, { readonly: true });
    }
    return db;
}

export function getIndex(): IReportIndex {
    const database = getDatabase();
    
    // Query election index
    const electionsQuery = database.prepare(`
        SELECT path, jurisdiction_name, election_name, date
        FROM election_index
        ORDER BY date DESC
    `);
    
    const elections = electionsQuery.all() as Array<{
        path: string;
        jurisdiction_name: string;
        election_name: string;
        date: string;
    }>;
    
    // Query contest summaries for each election
    const contestsQuery = database.prepare(`
        SELECT office, office_name, name, winner, num_candidates, num_rounds, ballot_count
        FROM contest_summaries
        WHERE election_path = ?
        ORDER BY office_name
    `);
    
    const result: IReportIndex = {
        elections: elections.map(election => ({
            path: election.path,
            jurisdictionName: election.jurisdiction_name,
            electionName: election.election_name,
            date: election.date,
            contests: contestsQuery.all(election.path).map((contest: any) => ({
                office: contest.office,
                officeName: contest.office_name,
                name: contest.name,
                winner: contest.winner,
                numCandidates: contest.num_candidates,
                numRounds: contest.num_rounds
            }))
        }))
    };
    
    return result;
}

export function getReport(path: string): IContestReport {
    const database = getDatabase();
    
    const reportQuery = database.prepare(`
        SELECT report_json
        FROM contest_reports
        WHERE path = ?
    `);
    
    const row = reportQuery.get(path) as { report_json: string } | undefined;
    
    if (!row) {
        throw new Error(`Report not found for path: ${path}`);
    }
    
    return JSON.parse(row.report_json) as IContestReport;
}

// Gracefully close database connection
export function closeDatabase() {
    if (db) {
        db.close();
        db = null;
    }
}

// Close database on process exit
process.on('exit', closeDatabase);
process.on('SIGINT', closeDatabase);
process.on('SIGTERM', closeDatabase);
