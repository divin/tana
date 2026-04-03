<script lang="ts">
	interface Props {
		value?: string;
		options: Array<{ value: string; label: string }>;
		label?: string;
		onChange?: (value: string) => void;
	}

	let { value = '', options = [], label = 'Sort by:', onChange }: Props = $props();

	function handleChange(e: Event) {
		const target = e.target as HTMLSelectElement;
		onChange?.(target.value);
	}
</script>

<div class="sort-controls">
	<label for="sort">{label}</label>
	<select id="sort" bind:value onchange={handleChange}>
		{#each options as option}
			<option value={option.value}>{option.label}</option>
		{/each}
	</select>
</div>

<style>
	.sort-controls {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	label {
		font-weight: 500;
		color: var(--text-secondary);
	}

	select {
		padding: 8px 12px;
		border: 1px solid var(--border-color);
		border-radius: 6px;
		font-size: 14px;
		background: var(--input-bg);
		color: var(--text-primary);
		cursor: pointer;
		transition: border-color 0.2s ease;
	}

	select:focus {
		outline: none;
		border-color: var(--accent-primary);
	}

	@media (min-width: 640px) {
		select:hover {
			border-color: var(--accent-primary);
		}
	}
</style>
