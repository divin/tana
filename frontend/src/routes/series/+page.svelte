<script lang="ts">
    import { onMount } from "svelte";
    import { apiClient, type TVSeries } from "$lib/api/client";
    import SortControls from "$lib/components/SortControls.svelte";
    import MediaGrid from "$lib/components/MediaGrid.svelte";

    let series = $state<TVSeries[]>([]);
    let error = $state<string | null>(null);
    let loading = $state(true);
    let sortBy = $state<string>("date");

    onMount(async () => {
        try {
            series = await apiClient.getSeries();
        } catch (err) {
            error =
                err instanceof Error ? err.message : "Failed to load series";
        } finally {
            loading = false;
        }
    });

    function getSortedSeries(): TVSeries[] {
        const sorted = [...series];
        switch (sortBy as "date" | "title" | "rating" | "status") {
            case "title":
                return sorted.sort((a, b) => a.title.localeCompare(b.title));
            case "rating":
                return sorted.sort((a, b) => (b.rating || 0) - (a.rating || 0));
            case "status":
                return sorted.sort((a, b) =>
                    (a.status || "").localeCompare(b.status || ""),
                );
            case "date":
            default:
                return sorted.sort(
                    (a, b) =>
                        new Date(b.started_date).getTime() -
                        new Date(a.started_date).getTime(),
                );
        }
    }

    function handleEdit(media: any) {
        if ("started_date" in media && !("watched_date" in media)) {
            window.location.href = `/series/${media.id}/edit`;
        }
    }

    async function handleDelete(media: any) {
        if ("started_date" in media && !("watched_date" in media)) {
            if (!confirm(`Delete "${media.title}"?`)) return;

            try {
                await apiClient.deleteSeries(media.id);
                series = series.filter((s) => s.id !== media.id);
            } catch (err) {
                error =
                    err instanceof Error
                        ? err.message
                        : "Failed to delete series";
            }
        }
    }

    function handleAdd() {
        window.location.href = "/series/add";
    }
</script>

<div class="series-page">
    <div class="header">
        <h1>TV Series</h1>
        <button class="add-button" onclick={handleAdd}>+ Add Series</button>
    </div>

    {#if loading}
        <div class="loading">Loading TV series...</div>
    {:else if error}
        <div class="error">{error}</div>
    {:else if series.length === 0}
        <div class="empty-state">
            <p>No TV series added yet. Start tracking your favorite shows!</p>
            <button class="empty-action" onclick={handleAdd}
                >Add Your First Series</button
            >
        </div>
    {:else}
        <div class="controls">
            <SortControls
                value={sortBy}
                options={[
                    { value: "date", label: "Date Started" },
                    { value: "title", label: "Title" },
                    { value: "rating", label: "Rating" },
                    { value: "status", label: "Status" },
                ]}
                onChange={(value) => {
                    sortBy = value;
                }}
            />
            <div class="count">
                Showing {series.length} series{series.length !== 1 ? "" : ""}
            </div>
        </div>

        <MediaGrid
            items={getSortedSeries()}
            onEdit={handleEdit}
            onDelete={handleDelete}
        />
    {/if}
</div>

<style>
    .series-page {
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
        .series-page {
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
