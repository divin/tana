<script lang="ts">
    import { page } from "$app/stores";
    import { onMount } from "svelte";
    import { apiClient, type Book } from "$lib/api/client";
    import BookForm from "$lib/components/forms/BookForm.svelte";

    let book = $state<Book | null>(null);
    let loading = $state(true);
    let error = $state<string | null>(null);

    onMount(async () => {
        try {
            if (!$page.params.id) {
                throw new Error("No book ID provided");
            }
            const id = parseInt($page.params.id, 10);
            book = await apiClient.getBook(id);
        } catch (err) {
            error = err instanceof Error ? err.message : "Failed to load book";
        } finally {
            loading = false;
        }
    });

    function handleSave() {
        window.location.href = "/books";
    }

    function handleCancel() {
        window.history.back();
    }
</script>

<div class="page">
    <div class="header">
        <button class="back-button" onclick={handleCancel}>← Back</button>
    </div>

    {#if loading}
        <div class="loading">Loading book...</div>
    {:else if error}
        <div class="error-message">{error}</div>
    {:else if book}
        <BookForm {book} onSave={handleSave} onCancel={handleCancel} />
    {:else}
        <div class="error-message">Book not found</div>
    {/if}
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

    .loading,
    .error-message {
        text-align: center;
        padding: 40px 16px;
        font-size: 16px;
        color: var(--text-secondary);
    }

    .error-message {
        background-color: var(--color-error-bg);
        color: var(--color-error-text);
        padding: 16px;
        border-radius: 8px;
        margin-bottom: 16px;
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
