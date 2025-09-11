import { error } from '@sveltejs/kit';
import { getIndex } from '$lib/server/reports.js';

export const load = async () => {
	try {
		const index = getIndex();
		return {
			elections: index.elections
		};
	} catch (err) {
		console.error('Error loading index:', err);
		throw error(500, 'Failed to load election index');
	}
};
