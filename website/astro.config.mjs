// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
	site: 'https://aztibase.com',
	integrations: [
		starlight({
			title: 'Aztibase Network',
			description: 'AI-native, server-independent Layer-1 blockchain built in Rust',
			logo: {
				light: './src/assets/logo-light.svg',
				dark: './src/assets/logo-dark.svg',
				replacesTitle: false,
			},
			social: [
				{ icon: 'github', label: 'GitHub', href: 'https://github.com/aztibase' },
				{ icon: 'x.com', label: 'Twitter', href: 'https://x.com/aztibase' },
			],
			components: {
				Header: './src/components/Header.astro',
			},
			customCss: ['./src/styles/custom.css'],
			head: [
				{
					tag: 'link',
					attrs: {
						rel: 'preconnect',
						href: 'https://fonts.googleapis.com',
					},
				},
				{
					tag: 'link',
					attrs: {
						rel: 'preconnect',
						href: 'https://fonts.gstatic.com',
						crossorigin: true,
					},
				},
				{
					tag: 'link',
					attrs: {
						rel: 'stylesheet',
						href: 'https://fonts.googleapis.com/css2?family=Instrument+Sans:wght@400;500;600;700&family=Jura:wght@400;500;600;700&display=swap',
					},
				},
				{
					tag: 'link',
					attrs: {
						rel: 'stylesheet',
						href: 'https://fonts.googleapis.com/css2?family=Fira+Code:wght@400;500&display=swap',
					},
				},
				{
					tag: 'meta',
					attrs: {
						property: 'og:image',
						content: '/og-image.png',
					},
				},
			],
			sidebar: [
				{
					label: 'Getting Started',
					items: [
						{ label: 'Introduction', slug: 'guides/introduction' },
						{ label: 'Quick Start', slug: 'guides/quickstart' },
						{ label: 'Run a Full Node', slug: 'guides/run-fullnode' },
						{ label: 'Run a Validator', slug: 'guides/run-validator' },
					],
				},
				{
					label: 'Architecture',
					items: [
						{ label: 'Overview', slug: 'architecture/overview' },
						{ label: 'Consensus', slug: 'architecture/consensus' },
						{ label: 'Networking', slug: 'architecture/networking' },
						{ label: 'Tokenomics', slug: 'architecture/tokenomics' },
						{ label: 'AI Integration', slug: 'architecture/ai-integration' },
					],
				},
				{
					label: 'API Reference',
					items: [
						{ label: 'JSON-RPC API', slug: 'api/rpc' },
						{ label: 'Transaction Types', slug: 'api/transactions' },
					],
				},
				{
					label: 'Tools',
					items: [
						{ label: 'Testnet Faucet', slug: 'tools/faucet' },
						{ label: 'Deploy Contracts', slug: 'tools/deploy' },
					],
				},
				{
					label: 'Network',
					items: [
						{ label: 'Testnet Guide', slug: 'network/testnet' },
						{ label: 'Genesis Ceremony', slug: 'network/genesis-ceremony' },
						{ label: 'VPS & Hardware', slug: 'network/vps-guide' },
					],
				},
				{
					label: 'Decisions',
					autogenerate: { directory: 'decisions' },
				},
			],
		}),
	],
});
