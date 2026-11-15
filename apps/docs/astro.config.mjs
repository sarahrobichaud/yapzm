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
				{
					label: 'Getting started',
					items: [
						{ label: 'About', slug: 'about' },
						{ label: 'Installation', slug: 'installation' },
					],
				},
				{
					label: 'Development',
					items: [{ label: 'Roadmap', slug: 'development/roadmap' }],
				},
				{
					label: 'Reference',
					items: [
						{ label: 'Configuration', slug: 'reference/configuration' },
						{ label: 'CLI', slug: 'reference/cli' },
					],
				},
			],
		}),
	],
});
