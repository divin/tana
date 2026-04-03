<script lang="ts">
	import BookForm from '$lib/components/forms/BookForm.svelte';

	function handleSave() {
		window.location.href = '/books';
	}

	function handleCancel() {
		window.history.back();
	}
</script>

<div class="page">
	<div class="header">
		<button class="back-button" onclick={handleCancel}>← Back</button>
	</div>
	<BookForm onSave={handleSave} onCancel={handleCancel} />
</div>

<style>
	.page {
		padding: 16px;
		max-width: 1200px;
		margin: 0 auto;
		min-height: 100vh;
		background: var(--bg-primary);
	}

	.header {
		margin-bottom: 24px;
	}

	.back-button {
		padding: 8px 16px;
		background: var(--card-bg);
		color: var(--text-primary);
		border: 1px solid var(--border-color);
		border-radius: 8px;
		font-weight: 500;
		cursor: pointer;
		transition: background-color 0.2s ease;
	}

	.back-button:active {
		background: var(--bg-secondary);
	}

	@media (min-width: 640px) {
		.page {
			padding: 24px;
		}

		.back-button:hover {
			background: var(--bg-secondary);
		}
	}
</style>
