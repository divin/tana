<script lang="ts">
	import { onMount } from 'svelte';
	import StatBox from '$lib/components/StatBox.svelte';
	import MediaGrid from '$lib/components/MediaGrid.svelte';
	import { apiClient, type Stats, type Movie, type TVSeries, type Book } from '$lib/api/client';

	let stats = $state<Stats | null>(null);
	let recentMedia = $state<{ movies: Movie[]; series: TVSeries[]; books: Book[] } | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(true);

	onMount(async () => {
		try {
			const [statsData, recentData] = await Promise.all([
				apiClient.getStats(),
				apiClient.getRecentlyAdded(5)
			]);
			stats = statsData;
			recentMedia = recentData;
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load dashboard data';
		} finally {
			loading = false;
		}
	});

	function getMediaType(media: Movie | TVSeries | Book): 'movie' | 'series' | 'book' {
		if ('poster_path' in media && 'watched_date' in media) return 'movie';
		if ('poster_path' in media && 'started_date' in media) return 'series';
		return 'book';
	}

	function handleEdit(media: Movie | TVSeries | Book) {
		const type = getMediaType(media);
		window.location.href = `/${type === 'movie' ? 'movies' : type === 'series' ? 'series' : 'books'}/${media.id}/edit`;
	}

	async function handleDelete(media: Movie | TVSeries | Book) {
		if (!confirm(`Delete "${media.title}"?`)) return;

		try {
			const type = getMediaType(media);
			if (type === 'movie') {
				await apiClient.deleteMovie(media.id);
			} else if (type === 'series') {
				await apiClient.deleteSeries(media.id);
			} else {
				await apiClient.deleteBook(media.id);
			}
			location.reload();
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to delete media';
		}
	}
</script>

<div class="dashboard">
	{#if loading}
		<div class="loading">Loading dashboard...</div>
	{:else if error}
		<div class="error">{error}</div>
	{:else}
		<!-- Welcome Section -->
		<section class="welcome-section">
			<h1 class="welcome-title">Welcome to Tana</h1>
			<p class="welcome-subtitle">Track your favorite movies, books, and TV series all in one place</p>
		</section>

		<!-- Stats Bento Grid -->
		{#if stats}
			<section class="stats-section">
				<div class="stats-grid">
					<StatBox label="Total" value={stats.total_count} isTotal={true} />
					<StatBox label="Books" value={stats.total_books} href="/books" />
					<StatBox label="Movies" value={stats.total_movies} href="/movies" />
					<StatBox label="Series" value={stats.total_series} href="/series" />
				</div>
			</section>
		{/if}

		<!-- Recently Added Section -->
		{#if recentMedia && (recentMedia.movies.length > 0 || recentMedia.series.length > 0 || recentMedia.books.length > 0)}
			<section class="recent-section">
				<h2 class="section-title">Recently Added</h2>
				<MediaGrid
					items={[...recentMedia.movies, ...recentMedia.series, ...recentMedia.books].sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime()).slice(0, 6)}
					onEdit={handleEdit}
					onDelete={handleDelete}
				/>
			</section>
		{/if}

		{#if !recentMedia || (recentMedia.movies.length === 0 && recentMedia.series.length === 0 && recentMedia.books.length === 0)}
			<div class="empty-state">
				<p>No media added yet. Start tracking your favorite movies, TV shows, and books!</p>
				<a href="/movies" class="empty-action">Add Media</a>
			</div>
		{/if}
	{/if}
</div>

<style>
	.dashboard {
		padding: 16px;
		max-width: 1200px;
		margin: 0 auto;
	}

	/* Welcome Section */
	.welcome-section {
		margin-bottom: 32px;
	}

	.welcome-title {
		font-size: 32px;
		font-weight: 700;
		color: var(--text-primary);
		margin: 0 0 8px 0;
	}

	.welcome-subtitle {
		font-size: 16px;
		color: var(--text-secondary);
		margin: 0;
		font-weight: 400;
	}

	.loading,
	.error {
		text-align: center;
		padding: 40px 16px;
		font-size: 16px;
		color: var(--text-secondary);
	}

	.error {
		background-color: var(--color-error-bg);
		color: var(--color-error-text);
		padding: 16px;
		border-radius: 8px;
		margin-bottom: 16px;
	}

	/* Stats Section */
	.stats-section {
		margin-bottom: 32px;
	}

	.stats-grid {
		display: grid;
		grid-template-columns: 1fr;
		gap: 12px;
	}

	/* Recently Added Section */
	.recent-section {
		margin-bottom: 24px;
	}

	.section-title {
		font-size: 18px;
		font-weight: 600;
		margin-bottom: 16px;
		color: var(--text-primary);
	}

	.empty-state {
		text-align: center;
		padding: 60px 16px;
		color: var(--text-secondary);
	}

	.empty-action {
		display: inline-block;
		margin-top: 16px;
		padding: 12px 24px;
		background: var(--accent-primary);
		color: white;
		border-radius: 8px;
		text-decoration: none;
		font-weight: 500;
		transition: background-color 0.2s ease;
	}

	.empty-action:active {
		background: var(--accent-primary-dark);
	}

	/* Tablet and Desktop Styles */
	@media (min-width: 640px) {
		.dashboard {
			padding: 24px;
		}

		.welcome-title {
			font-size: 40px;
		}

		.welcome-subtitle {
			font-size: 18px;
		}

		.stats-grid {
			grid-template-columns: repeat(2, 1fr);
		}
	}

	@media (min-width: 1024px) {
		.stats-grid {
			grid-template-columns: repeat(4, 1fr);
		}
	}
</style>
