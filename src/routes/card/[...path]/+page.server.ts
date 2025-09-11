import { error } from '@sveltejs/kit';
import { getReport } from '$lib/server/reports.js';

export const load = async ({ params }) => {
	try {
		const path = params.path;
		const report = getReport(path);
		
		if (!report) {
			throw error(404, 'Report not found');
		}
		
		return {
			report
		};
	} catch (err) {
		console.error('Error loading report for card:', err);
		throw error(404, 'Report not found');
	}
};
