<script lang="ts">
	import { apiClient, type Book } from '$lib/api/client';

	interface Props {
		book?: Book;
		onSave?: () => void;
		onCancel?: () => void;
	}

	let { book, onSave, onCancel }: Props = $props();

	let title = $state(book?.title || '');
	let author = $state(book?.author || '');
	let isbn = $state(book?.isbn || '');
	let genre = $state(book?.genre || '');
	let pages = $state(book?.pages?.toString() || '');
	let rating = $state(book?.rating?.toString() || '');
	let startedDate = $state(book?.started_date || '');
	let completedDate = $state(book?.completed_date || '');
	let notes = $state(book?.notes || '');
	let coverPath = $state(book?.cover_path || '');
	let error = $state<string | null>(null);
	let saving = $state(false);

	const isEditMode = !!book;

	async function handleSubmit() {
		error = null;

		// Validation
		if (!title.trim()) {
			error = 'Title is required';
			return;
		}

		if (!author.trim()) {
			error = 'Author is required';
			return;
		}

		if (!completedDate) {
			error = 'Completed date is required';
			return;
		}

		saving = true;

		try {
			const bookData: Omit<Book, 'id' | 'created_at' | 'updated_at'> = {
				title: title.trim(),
				author: author.trim(),
				isbn: isbn.trim() || undefined,
				genre: genre.trim() || undefined,
				pages: pages ? parseInt(pages, 10) : undefined,
				rating: rating ? parseFloat(rating) : undefined,
				started_date: startedDate || undefined,
				completed_date: completedDate,
				notes: notes.trim() || undefined,
				cover_path: coverPath.trim() || undefined,
			};

			if (isEditMode && book) {
				await apiClient.updateBook(book.id, bookData);
			} else {
				await apiClient.createBook(bookData);
			}

			onSave?.();
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to save book';
		} finally {
			saving = false;
		}
	}

	function handleCancel() {
		onCancel?.();
	}
</script>

<div class="form-container">
	<h2>{isEditMode ? 'Edit Book' : 'Add New Book'}</h2>

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
				placeholder="e.g., The Great Gatsby"
				disabled={saving}
			/>
		</div>

		<div class="form-group">
			<label for="author">Author *</label>
			<input
				id="author"
				type="text"
				bind:value={author}
				placeholder="e.g., F. Scott Fitzgerald"
				disabled={saving}
			/>
		</div>

		<div class="form-row">
			<div class="form-group">
				<label for="isbn">ISBN</label>
				<input
					id="isbn"
					type="text"
					bind:value={isbn}
					placeholder="e.g., 978-0743273565"
					disabled={saving}
				/>
			</div>

			<div class="form-group">
				<label for="genre">Genre</label>
				<input
					id="genre"
					type="text"
					bind:value={genre}
					placeholder="e.g., Fiction"
					disabled={saving}
				/>
			</div>
		</div>

		<div class="form-row">
			<div class="form-group">
				<label for="pages">Pages</label>
				<input
					id="pages"
					type="number"
					bind:value={pages}
					placeholder="e.g., 180"
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
					placeholder="e.g., 8.5"
					min="0"
					max="10"
					step="0.5"
					disabled={saving}
				/>
			</div>
		</div>

		<div class="form-row">
			<div class="form-group">
				<label for="started-date">Started Date</label>
				<input
					id="started-date"
					type="date"
					bind:value={startedDate}
					disabled={saving}
				/>
			</div>

			<div class="form-group">
				<label for="completed-date">Completed Date *</label>
				<input
					id="completed-date"
					type="date"
					bind:value={completedDate}
					disabled={saving}
				/>
			</div>
		</div>

		<div class="form-group">
			<label for="cover">Cover URL</label>
			<input
				id="cover"
				type="url"
				bind:value={coverPath}
				placeholder="https://example.com/cover.jpg"
				disabled={saving}
			/>
		</div>

		<div class="form-group">
			<label for="notes">Notes</label>
			<textarea
				id="notes"
				bind:value={notes}
				placeholder="Add any notes about this book..."
				rows="4"
				disabled={saving}
			/>
		</div>

		<div class="form-actions">
			<button type="button" class="btn-secondary" onclick={handleCancel} disabled={saving}>
				Cancel
			</button>
			<button type="submit" class="btn-primary" disabled={saving}>
				{saving ? 'Saving...' : isEditMode ? 'Update Book' : 'Add Book'}
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

	input:focus,
	textarea:focus {
		outline: none;
		border-color: var(--input-focus-border);
		box-shadow: 0 0 0 3px var(--input-focus-shadow);
	}

	input:disabled,
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
