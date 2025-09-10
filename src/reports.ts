import type { IReportIndex, IContestReport } from "./report_types"
import * as sqliteReports from './reports_sqlite'

const { RANKED_VOTE_REPORTS_DB } = process.env

export function getIndex(): IReportIndex {
    if (!RANKED_VOTE_REPORTS_DB) {
        throw new Error('RANKED_VOTE_REPORTS_DB environment variable is required');
    }
    
    try {
        return sqliteReports.getIndex();
    } catch (error) {
        throw new Error(`Failed to load reports from SQLite database: ${error.message}`);
    }
}

export function getReport(path: string): IContestReport {
    if (!RANKED_VOTE_REPORTS_DB) {
        throw new Error('RANKED_VOTE_REPORTS_DB environment variable is required');
    }
    
    try {
        return sqliteReports.getReport(path);
    } catch (error) {
        throw new Error(`Failed to load report for ${path}: ${error.message}`);
    }
}