import { sveltekit } from '@sveltejs/kit/vite';

/** @type {import('vite').UserConfig} */
const config = {
	plugins: [sveltekit()],
	server: {
		fs: {
			// Allow serving files from the report_pipeline directory
			allow: ['..', 'report_pipeline']
		}
	}
};

export default config;
