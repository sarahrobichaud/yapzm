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
			customCss: ['./src/custom.css'],
			sidebar: [{ label: 'About', slug: 'about' }],
		}),
	],
});
