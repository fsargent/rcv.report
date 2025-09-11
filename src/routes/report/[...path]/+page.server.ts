import { error } from '@sveltejs/kit';
import { getReport } from '$lib/server/reports.js';

export const load = async ({ params }) => {
	try {
		const path = params.path;
		console.log('Loading report for path:', path);
		
		const report = getReport(path);
		console.log('Report loaded:', report ? 'success' : 'null');
		
		if (!report) {
			console.error('Report is null for path:', path);
			throw error(404, 'Report not found');
		}
		
		// Ensure report has required structure
		if (!report.info) {
			console.error('Report missing info field:', report);
			throw error(500, 'Invalid report structure');
		}
		
		return {
			report,
			path
		};
	} catch (err) {
		console.error('Error loading report for path:', params.path, err);
		throw error(404, 'Report not found');
	}
};
