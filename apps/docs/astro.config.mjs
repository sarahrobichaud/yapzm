// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

// https://astro.build/config
export default defineConfig({
	integrations: [
		starlight({
			title: 'YAPZM',
			description: 'Yet another Project Zomboid Manager',
			favicon: '/favicon.svg',
			sidebar: [
				{ label: 'About', slug: 'about' },
				{ label: 'Installation', slug: 'installation' },
				{ label: 'Configuration', slug: 'reference/configuration' },
				{
					label: 'Clients',
					items: [{ label: 'CLI', slug: 'reference/cli' }],
				},
			],
		}),
	],
});
