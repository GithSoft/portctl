// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

// https://astro.build/config
export default defineConfig({
	site: 'https://portctl.githsoft.com',
	integrations: [
		starlight({
			title: 'portctl',
			description: 'Lightweight, zero-daemon, local-first Port Manager and automated Edge Ingress CLI in pure Rust.',
			social: [
				{ icon: 'github', label: 'GitHub', href: 'https://github.com/tackelua/portctl' }
			],
			customCss: [
				'./src/styles/custom.css'
			],
			editLink: {
				baseUrl: 'https://github.com/tackelua/portctl/edit/main/website/'
			},
			sidebar: [
				{
					label: 'Getting Started',
					items: [
						{ label: 'Quickstart', slug: 'getting-started/quickstart' },
						{ label: 'Installation', slug: 'getting-started/installation' },
					],
				},
				{
					label: 'Core Guides',
					items: [
						{ label: 'Cloudflare Tunnel Ingress', slug: 'guides/cloudflare-tunnel' },
						{ label: 'Process Hunter (Kill)', slug: 'guides/process-hunter' },
						{ label: 'AI Coding Agents & MCP', slug: 'guides/ai-agent-mcp' },
					],
				},
				{
					label: 'Reference',
					items: [
						{ label: 'CLI Command Reference', slug: 'reference/cli-commands' },
						{ label: 'Architecture & Locking', slug: 'reference/architecture' },
						{ label: 'Master Roadmap', slug: 'reference/roadmap' },
					],
				},
			],
		}),
	],
});
