<script lang="ts">
    import { onMount } from "svelte";
    import { apiClient, type Book } from "$lib/api/client";
    import SortControls from "$lib/components/SortControls.svelte";
    import MediaGrid from "$lib/components/MediaGrid.svelte";

    let books = $state<Book[]>([]);
    let error = $state<string | null>(null);
    let loading = $state(true);
    let sortBy = $state<string>("date");

    onMount(async () => {
        try {
            books = await apiClient.getBooks();
        } catch (err) {
            error = err instanceof Error ? err.message : "Failed to load books";
        } finally {
            loading = false;
        }
    });

    function getSortedBooks(): Book[] {
        const sorted = [...books];
        switch (sortBy as "date" | "title" | "author" | "rating") {
            case "title":
                return sorted.sort((a, b) => a.title.localeCompare(b.title));
            case "author":
                return sorted.sort((a, b) => a.author.localeCompare(b.author));
            case "rating":
                return sorted.sort((a, b) => (b.rating || 0) - (a.rating || 0));
            case "date":
            default:
                return sorted.sort(
                    (a, b) =>
                        new Date(b.completed_date).getTime() -
                        new Date(a.completed_date).getTime(),
                );
        }
    }

    function handleEdit(media: any) {
        if ("author" in media && "completed_date" in media) {
            window.location.href = `/books/${media.id}/edit`;
        }
    }

    async function handleDelete(media: any) {
        if ("author" in media && "completed_date" in media) {
            if (!confirm(`Delete "${media.title}"?`)) return;

            try {
                await apiClient.deleteBook(media.id);
                books = books.filter((b) => b.id !== media.id);
            } catch (err) {
                error =
                    err instanceof Error
                        ? err.message
                        : "Failed to delete book";
            }
        }
    }

    function handleAdd() {
        window.location.href = "/books/add";
    }
</script>

<div class="books-page">
    <div class="header">
        <h1>Books</h1>
        <button class="add-button" onclick={handleAdd}>+ Add Book</button>
    </div>

    {#if loading}
        <div class="loading">Loading books...</div>
    {:else if error}
        <div class="error">{error}</div>
    {:else if books.length === 0}
        <div class="empty-state">
            <p>No books added yet. Start tracking your favorite books!</p>
            <button class="empty-action" onclick={handleAdd}
                >Add Your First Book</button
            >
        </div>
    {:else}
        <div class="controls">
            <SortControls
                value={sortBy}
                options={[
                    { value: "date", label: "Date Completed" },
                    { value: "title", label: "Title" },
                    { value: "author", label: "Author" },
                    { value: "rating", label: "Rating" },
                ]}
                onChange={(value) => {
                    sortBy = value;
                }}
            />
            <div class="count">
                Showing {books.length} book{books.length !== 1 ? "s" : ""}
            </div>
        </div>

        <MediaGrid
            items={getSortedBooks()}
            onEdit={handleEdit}
            onDelete={handleDelete}
        />
    {/if}
</div>

<style>
    .books-page {
        padding: 16px;
        max-width: 1200px;
        margin: 0 auto;
    }

    .header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 24px;
        gap: 12px;
    }

    .header h1 {
        font-size: 28px;
        font-weight: 700;
        color: var(--text-primary);
        margin: 0;
    }

    .add-button {
        padding: 10px 16px;
        background: var(--accent-primary);
        color: white;
        border: none;
        border-radius: 8px;
        font-weight: 500;
        cursor: pointer;
        transition: background-color 0.2s ease;
        white-space: nowrap;
    }

    .add-button:active {
        background: var(--accent-primary-dark);
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
        border: none;
        border-radius: 8px;
        font-weight: 500;
        cursor: pointer;
        transition: background-color 0.2s ease;
    }

    .empty-action:active {
        background: var(--accent-primary-dark);
    }

    .controls {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 24px;
        gap: 12px;
        flex-wrap: wrap;
    }

    .count {
        font-size: 14px;
        color: var(--text-secondary);
    }

    @media (min-width: 640px) {
        .books-page {
            padding: 24px;
        }

        .header h1 {
            font-size: 32px;
        }

        .add-button:hover {
            background: var(--accent-primary-dark);
        }
    }
</style>
