<script lang="ts">
    interface Props {
        value?: string;
        placeholder?: string;
        loading?: boolean;
        onSearch?: (query: string) => void;
        onChange?: (value: string) => void;
    }

    let {
        value = "",
        placeholder = "Search...",
        loading = false,
        onSearch,
        onChange,
    }: Props = $props();

    function handleKeyDown(event: KeyboardEvent) {
        if (event.key === "Enter") {
            event.preventDefault();
            onSearch?.(value);
        }
    }

    function handleSearch() {
        onSearch?.(value);
    }

    function handleInput(e: Event) {
        const target = e.target as HTMLInputElement;
        value = target.value;
        onChange?.(target.value);
    }
</script>

<div class="search-box">
    <input
        type="text"
        {placeholder}
        bind:value
        onkeydown={handleKeyDown}
        oninput={handleInput}
        class="search-input"
        disabled={loading}
    />
    <button class="search-button" onclick={handleSearch} disabled={loading}>
        {#if loading}
            Searching...
        {:else}
            Search
        {/if}
    </button>
</div>

<style>
    .search-box {
        display: flex;
        gap: 8px;
        width: 100%;
    }

    .search-input {
        flex: 1;
        padding: 12px 16px;
        border: 1px solid var(--input-border);
        border-radius: 8px;
        font-size: 16px;
        font-family: inherit;
        background-color: var(--input-bg);
        color: var(--input-text);
        transition: border-color 0.2s ease;
    }

    .search-input:focus {
        outline: none;
        border-color: var(--input-focus-border);
        box-shadow: 0 0 0 3px var(--input-focus-shadow);
    }

    .search-input:disabled {
        opacity: 0.6;
        cursor: not-allowed;
    }

    .search-button {
        padding: 12px 24px;
        background: var(--btn-primary-bg);
        color: var(--text-primary);
        border: none;
        border-radius: 8px;
        font-weight: 500;
        cursor: pointer;
        transition: background-color 0.2s ease;
        white-space: nowrap;
    }

    .search-button:active {
        background: var(--btn-primary-active);
    }

    .search-button:disabled {
        background: var(--btn-primary-disabled);
        cursor: not-allowed;
    }

    @media (min-width: 640px) {
        .search-button:hover:not(:disabled) {
            background: var(--btn-primary-hover);
        }
    }
</style>
