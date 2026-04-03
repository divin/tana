<script lang="ts">
	import { page } from '$app/stores';

	type NavItem = {
		label: string;
		href: string;
		icon: string;
	};

	const navItems: NavItem[] = [
		{ label: 'Dashboard', href: '/', icon: '📊' },
		{ label: 'Movies', href: '/movies', icon: '🎬' },
		{ label: 'Books', href: '/books', icon: '📚' },
		{ label: 'TV', href: '/series', icon: '📺' },
		{ label: 'Search', href: '/search', icon: '🔍' }
	];

	function isActive(href: string): boolean {
		return $page.url.pathname === href;
	}
</script>

<nav class="bottom-nav">
	{#each navItems as item (item.href)}
		<a
			href={item.href}
			class="nav-item"
			class:active={isActive(item.href)}
			aria-label={item.label}
		>
			<span class="icon">{item.icon}</span>
			<span class="label">{item.label}</span>
		</a>
	{/each}
</nav>

<style>
	.bottom-nav {
		position: fixed;
		bottom: 0;
		left: 0;
		right: 0;
		display: flex;
		justify-content: space-around;
		align-items: stretch;
		background: var(--bg-navbar);
		border-top: 1px solid var(--border-color);
		box-shadow: 0 -2px 8px var(--shadow-dark);
		z-index: 100;
		height: 70px;
	}

	.nav-item {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 12px 8px;
		text-decoration: none;
		color: var(--text-secondary);
		font-size: 11px;
		font-weight: 500;
		transition: color 0.2s ease;
		border: none;
		background: none;
		cursor: pointer;
		gap: 4px;
	}

	.nav-item:active {
		background-color: var(--bg-secondary);
	}

	.nav-item.active {
		color: var(--accent-primary);
	}

	.icon {
		font-size: 24px;
		line-height: 1;
	}

	.label {
		text-align: center;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		width: 100%;
	}

	@media (min-width: 640px) {
		.bottom-nav {
			position: fixed;
			bottom: 0;
			left: 0;
			right: 0;
			border-top: 1px solid var(--border-color);
			border-bottom: none;
			box-shadow: 0 -2px 8px var(--shadow-dark);
			background: var(--bg-navbar);
			height: 60px;
		}

		.nav-item {
			padding: 14px 16px;
			flex-direction: row;
			gap: 8px;
			justify-content: flex-start;
			font-size: 14px;
		}

		.icon {
			font-size: 20px;
		}

		.label {
			white-space: normal;
		}
	}
</style>
