<script lang="ts">
    import { onMount } from "svelte";
    import { apiClient, type Movie } from "$lib/api/client";
    import SortControls from "$lib/components/SortControls.svelte";
    import MediaGrid from "$lib/components/MediaGrid.svelte";

    let movies = $state<Movie[]>([]);
    let error = $state<string | null>(null);
    let loading = $state(true);
    let sortBy = $state<string>("date");

    onMount(async () => {
        try {
            movies = await apiClient.getMovies();
        } catch (err) {
            error =
                err instanceof Error ? err.message : "Failed to load movies";
        } finally {
            loading = false;
        }
    });

    function getSortedMovies(): Movie[] {
        const sorted = [...movies];
        switch (sortBy as "date" | "title" | "rating") {
            case "title":
                return sorted.sort((a, b) => a.title.localeCompare(b.title));
            case "rating":
                return sorted.sort((a, b) => (b.rating || 0) - (a.rating || 0));
            case "date":
            default:
                return sorted.sort(
                    (a, b) =>
                        new Date(b.watched_date).getTime() -
                        new Date(a.watched_date).getTime(),
                );
        }
    }

    function handleEdit(media: any) {
        if ("watched_date" in media) {
            window.location.href = `/movies/${media.id}/edit`;
        }
    }

    async function handleDelete(media: any) {
        if ("watched_date" in media) {
            if (!confirm(`Delete "${media.title}"?`)) return;

            try {
                await apiClient.deleteMovie(media.id);
                movies = movies.filter((m) => m.id !== media.id);
            } catch (err) {
                error =
                    err instanceof Error
                        ? err.message
                        : "Failed to delete movie";
            }
        }
    }

    function handleAdd() {
        window.location.href = "/movies/add";
    }
</script>

<div class="movies-page">
    <div class="header">
        <h1>Movies</h1>
        <button class="add-button" onclick={handleAdd}>+ Add Movie</button>
    </div>

    {#if loading}
        <div class="loading">Loading movies...</div>
    {:else if error}
        <div class="error">{error}</div>
    {:else if movies.length === 0}
        <div class="empty-state">
            <p>No movies added yet. Start tracking your favorite movies!</p>
            <button class="empty-action" onclick={handleAdd}
                >Add Your First Movie</button
            >
        </div>
    {:else}
        <div class="controls">
            <SortControls
                value={sortBy}
                options={[
                    { value: "date", label: "Date Watched" },
                    { value: "title", label: "Title" },
                    { value: "rating", label: "Rating" },
                ]}
                onChange={(value) => {
                    sortBy = value;
                }}
            />
            <div class="count">
                Showing {movies.length} movie{movies.length !== 1 ? "s" : ""}
            </div>
        </div>

        <MediaGrid
            items={getSortedMovies()}
            onEdit={handleEdit}
            onDelete={handleDelete}
        />
    {/if}
</div>

<style>
    .movies-page {
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
        color: #fecaca;
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
        .movies-page {
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
