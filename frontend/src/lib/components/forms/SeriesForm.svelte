<script lang="ts">
	import { apiClient, type TVSeries } from '$lib/api/client';

	interface Props {
		series?: TVSeries;
		onSave?: () => void;
		onCancel?: () => void;
	}

	let { series, onSave, onCancel }: Props = $props();

	let title = $state(series?.title || '');
	let releaseYear = $state(series?.release_year?.toString() || '');
	let status = $state(series?.status || 'Watching');
	let totalSeasons = $state(series?.total_seasons?.toString() || '');
	let currentSeason = $state(series?.current_season?.toString() || '');
	let currentEpisode = $state(series?.current_episode?.toString() || '');
	let rating = $state(series?.rating?.toString() || '');
	let startedDate = $state(series?.started_date || '');
	let completedDate = $state(series?.completed_date || '');
	let notes = $state(series?.notes || '');
	let posterPath = $state(series?.poster_path || '');
	let error = $state<string | null>(null);
	let saving = $state(false);

	const isEditMode = !!series;
	const statusOptions = ['Watching', 'Completed', 'Dropped', 'Plan to Watch'];

	async function handleSubmit() {
		error = null;

		// Validation
		if (!title.trim()) {
			error = 'Title is required';
			return;
		}

		if (!startedDate) {
			error = 'Started date is required';
			return;
		}

		saving = true;

		try {
			const seriesData: Omit<TVSeries, 'id' | 'created_at' | 'updated_at'> = {
				title: title.trim(),
				release_year: releaseYear ? parseInt(releaseYear, 10) : undefined,
				status: status || undefined,
				total_seasons: totalSeasons ? parseInt(totalSeasons, 10) : undefined,
				current_season: currentSeason ? parseInt(currentSeason, 10) : undefined,
				current_episode: currentEpisode ? parseInt(currentEpisode, 10) : undefined,
				rating: rating ? parseFloat(rating) : undefined,
				started_date: startedDate,
				completed_date: completedDate || undefined,
				notes: notes.trim() || undefined,
				poster_path: posterPath.trim() || undefined,
			};

			if (isEditMode && series) {
				await apiClient.updateSeries(series.id, seriesData);
			} else {
				await apiClient.createSeries(seriesData);
			}

			onSave?.();
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to save series';
		} finally {
			saving = false;
		}
	}

	function handleCancel() {
		onCancel?.();
	}
</script>

<div class="form-container">
	<h2>{isEditMode ? 'Edit Series' : 'Add New Series'}</h2>

	{#if error}
		<div class="error-message">{error}</div>
	{/if}

	<form onsubmit={(e) => { e.preventDefault(); handleSubmit(); }}>
		<div class="form-group">
			<label for="title">Title *</label>
			<input
				id="title"
				type="text"
				bind:value={title}
				placeholder="e.g., Breaking Bad"
				disabled={saving}
			/>
		</div>

		<div class="form-row">
			<div class="form-group">
				<label for="release-year">Release Year</label>
				<input
					id="release-year"
					type="number"
					bind:value={releaseYear}
					placeholder="e.g., 2008"
					min="1800"
					max={new Date().getFullYear() + 5}
					disabled={saving}
				/>
			</div>

			<div class="form-group">
				<label for="status">Status</label>
				<select id="status" bind:value={status} disabled={saving}>
					{#each statusOptions as option}
						<option value={option}>{option}</option>
					{/each}
				</select>
			</div>
		</div>

		<div class="form-row">
			<div class="form-group">
				<label for="started-date">Started Date *</label>
				<input
					id="started-date"
					type="date"
					bind:value={startedDate}
					disabled={saving}
				/>
			</div>

			<div class="form-group">
				<label for="completed-date">Completed Date</label>
				<input
					id="completed-date"
					type="date"
					bind:value={completedDate}
					disabled={saving}
				/>
			</div>
		</div>

		<div class="form-row">
			<div class="form-group">
				<label for="total-seasons">Total Seasons</label>
				<input
					id="total-seasons"
					type="number"
					bind:value={totalSeasons}
					placeholder="e.g., 5"
					min="1"
					disabled={saving}
				/>
			</div>

			<div class="form-group">
				<label for="rating">Rating (0-10)</label>
				<input
					id="rating"
					type="number"
					bind:value={rating}
					placeholder="e.g., 9.5"
					min="0"
					max="10"
					step="0.5"
					disabled={saving}
				/>
			</div>
		</div>

		<div class="form-row">
			<div class="form-group">
				<label for="current-season">Current Season</label>
				<input
					id="current-season"
					type="number"
					bind:value={currentSeason}
					placeholder="e.g., 5"
					min="1"
					disabled={saving}
				/>
			</div>

			<div class="form-group">
				<label for="current-episode">Current Episode</label>
				<input
					id="current-episode"
					type="number"
					bind:value={currentEpisode}
					placeholder="e.g., 16"
					min="1"
					disabled={saving}
				/>
			</div>
		</div>

		<div class="form-group">
			<label for="poster">Poster URL</label>
			<input
				id="poster"
				type="url"
				bind:value={posterPath}
				placeholder="https://example.com/poster.jpg"
				disabled={saving}
			/>
		</div>

		<div class="form-group">
			<label for="notes">Notes</label>
			<textarea
				id="notes"
				bind:value={notes}
				placeholder="Add any notes about this series..."
				rows="4"
				disabled={saving}
			/>
		</div>

		<div class="form-actions">
			<button type="button" class="btn-secondary" onclick={handleCancel} disabled={saving}>
				Cancel
			</button>
			<button type="submit" class="btn-primary" disabled={saving}>
				{saving ? 'Saving...' : isEditMode ? 'Update Series' : 'Add Series'}
			</button>
		</div>
	</form>
</div>

<style>
	.form-container {
		max-width: 600px;
		margin: 0 auto;
		padding: 24px;
	}

	h2 {
		font-size: 24px;
		font-weight: 700;
		color: var(--text-primary);
		margin: 0 0 24px 0;
	}

	.error-message {
		background-color: var(--color-error-bg);
		color: var(--color-error-text);
		padding: 12px 16px;
		border-radius: 8px;
		margin-bottom: 16px;
		font-size: 14px;
	}

	form {
		display: flex;
		flex-direction: column;
		gap: 20px;
	}

	.form-group {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.form-row {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 16px;
	}

	label {
		font-size: 14px;
		font-weight: 500;
		color: var(--text-primary);
	}

	input,
	select,
	textarea {
		padding: 12px 16px;
		border: 1px solid var(--input-border);
		border-radius: 8px;
		font-size: 16px;
		font-family: inherit;
		background-color: var(--input-bg);
		color: var(--input-text);
		transition: border-color 0.2s ease;
	}

	select {
		cursor: pointer;
	}

	input:focus,
	select:focus,
	textarea:focus {
		outline: none;
		border-color: var(--input-focus-border);
		box-shadow: 0 0 0 3px var(--input-focus-shadow);
	}

	input:disabled,
	select:disabled,
	textarea:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	textarea {
		resize: vertical;
	}

	.form-actions {
		display: flex;
		gap: 12px;
		justify-content: flex-end;
		margin-top: 12px;
	}

	.btn-primary,
	.btn-secondary {
		padding: 12px 24px;
		border: none;
		border-radius: 8px;
		font-weight: 500;
		font-size: 16px;
		cursor: pointer;
		transition: background-color 0.2s ease;
	}

	.btn-primary {
		background: var(--accent-primary);
		color: white;
	}

	.btn-primary:active:not(:disabled) {
		background: var(--accent-primary-dark);
	}

	.btn-primary:disabled {
		background: var(--text-muted);
		cursor: not-allowed;
	}

	.btn-secondary {
		background: var(--card-bg);
		color: var(--text-primary);
		border: 1px solid var(--border-color);
	}

	.btn-secondary:active:not(:disabled) {
		background: var(--bg-secondary);
	}

	.btn-secondary:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	@media (min-width: 640px) {
		.btn-primary:hover:not(:disabled) {
			background: var(--accent-primary-dark);
		}

		.btn-secondary:hover:not(:disabled) {
			background: var(--bg-secondary);
		}
	}

	@media (max-width: 640px) {
		.form-row {
			grid-template-columns: 1fr;
		}
	}
</style>
