import type { IReportIndex, IContestReport } from "./report_types"
import * as sqliteReports from './reports_sqlite'

export function getIndex(): IReportIndex {
    try {
        return sqliteReports.getIndex();
    } catch (error) {
        throw new Error(`Failed to load reports from SQLite database: ${error.message}`);
    }
}

export function getReport(path: string): IContestReport {
    try {
        return sqliteReports.getReport(path);
    } catch (error) {
        throw new Error(`Failed to load report for ${path}: ${error.message}`);
    }
}