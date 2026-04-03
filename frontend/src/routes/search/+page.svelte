<script lang="ts">
    import {
        apiClient,
        type Movie,
        type TVSeries,
        type Book,
    } from "$lib/api/client";
    import SearchInput from "$lib/components/SearchInput.svelte";
    import MediaGrid from "$lib/components/MediaGrid.svelte";

    let searchQuery = $state("");
    let results = $state<{
        movies: Movie[];
        series: TVSeries[];
        books: Book[];
    } | null>(null);
    let error = $state<string | null>(null);
    let loading = $state(false);
    let searched = $state(false);

    async function handleSearch() {
        if (!searchQuery.trim()) {
            results = null;
            searched = false;
            error = null;
            return;
        }

        loading = true;
        error = null;
        searched = true;

        try {
            const response = await apiClient.search(searchQuery);
            console.log("Search API response:", response);

            // Validate and normalize the response structure
            if (response && typeof response === "object") {
                results = {
                    movies: Array.isArray(response.movies)
                        ? response.movies
                        : [],
                    series: Array.isArray(response.series)
                        ? response.series
                        : [],
                    books: Array.isArray(response.books) ? response.books : [],
                };
            } else {
                console.error("Invalid API response structure:", response);
                error = "Invalid response from server";
                results = { movies: [], series: [], books: [] };
            }
        } catch (err) {
            console.error("Search error:", err);
            error = err instanceof Error ? err.message : "Failed to search";
            results = { movies: [], series: [], books: [] };
        } finally {
            loading = false;
        }
    }

    function handleSearchKeyDown(event: KeyboardEvent) {
        if (event.key === "Enter") {
            handleSearch();
        }
    }

    function getMediaType(
        media: Movie | TVSeries | Book,
    ): "movie" | "series" | "book" {
        if ("poster_path" in media && "watched_date" in media) return "movie";
        if ("poster_path" in media && "started_date" in media) return "series";
        return "book";
    }

    function getMediaImage(media: Movie | TVSeries | Book): string {
        if ("poster_path" in media && media.poster_path) {
            return media.poster_path;
        }
        if ("cover_path" in media && media.cover_path) {
            return media.cover_path;
        }
        return "https://via.placeholder.com/150x225?text=No+Image";
    }

    function handleEdit(media: Movie | TVSeries | Book) {
        const type = getMediaType(media);
        window.location.href = `/${type === "movie" ? "movies" : type === "series" ? "series" : "books"}/${media.id}/edit`;
    }

    async function handleDelete(media: Movie | TVSeries | Book) {
        if (!confirm(`Delete "${media.title}"?`)) return;

        try {
            const type = getMediaType(media);
            if (type === "movie") {
                await apiClient.deleteMovie(media.id);
            } else if (type === "series") {
                await apiClient.deleteSeries(media.id);
            } else {
                await apiClient.deleteBook(media.id);
            }
            handleSearch();
        } catch (err) {
            error =
                err instanceof Error ? err.message : "Failed to delete media";
        }
    }
</script>

<div class="search-page">
    <div class="search-container">
        <h1>Search</h1>
        <div class="search-box">
            <SearchInput
                value={searchQuery}
                placeholder="Search movies, books, TV series..."
                {loading}
                onSearch={handleSearch}
                onChange={(value) => {
                    searchQuery = value;
                }}
            />
        </div>
    </div>

    {#if error}
        <div class="error">{error}</div>
    {/if}

    {#if loading}
        <div class="loading">Searching...</div>
    {:else if !searched}
        <div class="initial-state">
            <p>Enter a search term to find movies, books, and TV series.</p>
        </div>
    {:else if !results || (results.movies.length === 0 && results.series.length === 0 && results.books.length === 0)}
        <div class="no-results">
            <p>No results found for "{searchQuery}"</p>
        </div>
    {:else}
        <div class="results-container">
            {#if results.movies.length > 0}
                <section class="results-section">
                    <h2 class="section-title">
                        Movies ({results.movies.length})
                    </h2>
                    <MediaGrid
                        items={results.movies}
                        onEdit={handleEdit}
                        onDelete={handleDelete}
                    />
                </section>
            {/if}

            {#if results.series.length > 0}
                <section class="results-section">
                    <h2 class="section-title">
                        TV Series ({results.series.length})
                    </h2>
                    <MediaGrid
                        items={results.series}
                        onEdit={handleEdit}
                        onDelete={handleDelete}
                    />
                </section>
            {/if}

            {#if results.books.length > 0}
                <section class="results-section">
                    <h2 class="section-title">
                        Books ({results.books.length})
                    </h2>
                    <MediaGrid
                        items={results.books}
                        onEdit={handleEdit}
                        onDelete={handleDelete}
                    />
                </section>
            {/if}
        </div>
    {/if}
</div>

<style>
    .search-page {
        padding: 16px;
        max-width: 1200px;
        margin: 0 auto;
    }

    .search-container {
        margin-bottom: 32px;
    }

    .search-container h1 {
        font-size: 28px;
        font-weight: 700;
        color: var(--text-primary);
        margin: 0 0 16px 0;
    }

    .search-box {
        display: flex;
        gap: 8px;
        width: 100%;
    }

    .loading,
    .error,
    .no-results,
    .initial-state {
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

    .results-container {
        display: flex;
        flex-direction: column;
        gap: 40px;
    }

    .results-section {
        display: flex;
        flex-direction: column;
        gap: 16px;
    }

    .section-title {
        font-size: 18px;
        font-weight: 600;
        color: var(--text-primary);
        margin: 0;
    }

    @media (min-width: 640px) {
        .search-page {
            padding: 24px;
        }

        .search-container h1 {
            font-size: 32px;
        }
    }
</style>
