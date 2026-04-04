<script lang="ts">
    import type { Movie, TVSeries, Book, Media } from "$lib/api/client";

    interface Props {
        media: Media;
        onEdit?: (media: Media) => void;
        onDelete?: (media: Media) => void;
        isHovered?: boolean;
        onHoverChange?: (isHovered: boolean) => void;
    }

    let {
        media,
        onEdit,
        onDelete,
        isHovered = false,
        onHoverChange,
    }: Props = $props();

    let isTouched = $state(false);
    let isTouchDevice = $state(false);

    // Auto-close details when another card is tapped
    $effect(() => {
        const hovered = isHovered ?? false;
        if (!hovered && isTouched && isTouchDevice) {
            isTouched = false;
        }
    });

    function getMediaImage(item: Media): string {
        let imagePath: string | undefined;

        if ("poster_path" in item && item.poster_path) {
            imagePath = item.poster_path;
        } else if ("cover_path" in item && item.cover_path) {
            imagePath = item.cover_path;
        }

        if (imagePath) {
            if (
                imagePath.startsWith("http://") ||
                imagePath.startsWith("https://")
            ) {
                return imagePath;
            }
            return `/api/images/${imagePath}`;
        }

        return "https://via.placeholder.com/150x225?text=No+Image";
    }

    function getYear(item: Media): number | null {
        if ("release_year" in item) {
            const year = (item as Movie | TVSeries).release_year;
            if (year) {
                return year;
            }
        }
        return null;
    }

    function formatDate(dateString: string | undefined): string {
        if (!dateString) return "";
        return new Date(dateString).toLocaleDateString("en-US", {
            year: "numeric",
            month: "short",
            day: "numeric",
        });
    }

    function getMediaType(item: Media): "movie" | "series" | "book" {
        if ("poster_path" in item && "watched_date" in item) return "movie";
        if ("poster_path" in item && "started_date" in item) return "series";
        return "book";
    }

    function handlePointerEnter(e: PointerEvent) {
        // Only trigger hover on non-touch devices
        if (e.pointerType !== "touch") {
            onHoverChange?.(true);
        }
    }

    function handlePointerLeave(e: PointerEvent) {
        // Only trigger on non-touch devices, and keep overlay visible if tapped on touch device
        if (e.pointerType !== "touch" && !isTouched) {
            onHoverChange?.(false);
        }
    }

    function handlePointerDown(e: PointerEvent) {
        // Detect touch device and toggle overlay on touch
        if (e.pointerType === "touch") {
            isTouchDevice = true;
            isTouched = !isTouched;
            onHoverChange?.(isTouched);
        }
    }

    function handleOverlayClick(e: MouseEvent | PointerEvent) {
        // Close overlay when clicking outside the content area on mobile
        if (isTouched && isTouchDevice && e.target === e.currentTarget) {
            isTouched = false;
            onHoverChange?.(false);
        }
    }

    function handleButtonClick(e: MouseEvent) {
        e.stopPropagation();
    }

    // Determine whether to show overlay
    function shouldShowOverlay(): boolean {
        if (isTouchDevice) {
            return isTouched;
        }
        return isHovered;
    }
</script>

<div
    class="media-card"
    role="article"
    aria-label={media.title}
    onpointerenter={handlePointerEnter}
    onpointerleave={handlePointerLeave}
    onpointerdown={handlePointerDown}
>
    <div class="media-image-container" class:blurred={shouldShowOverlay()}>
        <img src={getMediaImage(media)} alt={media.title} class="media-image" />
    </div>

    <!-- Keep info visible but hidden - maintains card height -->
    <div class="media-info" class:invisible={shouldShowOverlay()}>
        <div class="title-year">
            <h3 class="media-title">
                {media.title}
                {#if getYear(media)}
                    <span class="year">({getYear(media)})</span>
                {/if}
            </h3>
        </div>
        {#if media.rating}
            <p class="media-rating">★ {media.rating.toFixed(1)}</p>
        {/if}
    </div>

    <!-- Overlay always in DOM, positioned absolutely -->
    <div
        class="hover-overlay"
        class:visible={shouldShowOverlay()}
        onclick={handleOverlayClick}
        role="presentation"
    >
        <div class="overlay-content">
            <h3 class="overlay-title">{media.title}</h3>

            {#if getMediaType(media) === "movie"}
                {@const movie = media as Movie}
                <div class="info-group">
                    {#if movie.director}
                        <div class="info-row">
                            <span class="info-label">Director:</span>
                            <span class="info-value">{movie.director}</span>
                        </div>
                    {/if}
                    {#if movie.release_year}
                        <div class="info-row">
                            <span class="info-label">Year:</span>
                            <span class="info-value">{movie.release_year}</span>
                        </div>
                    {/if}
                    {#if movie.rating}
                        <div class="info-row">
                            <span class="info-label">Rating:</span>
                            <span class="info-value rating-value"
                                >★ {movie.rating.toFixed(1)}</span
                            >
                        </div>
                    {/if}
                    {#if movie.watched_date}
                        <div class="info-row">
                            <span class="info-label">Watched:</span>
                            <span class="info-value"
                                >{formatDate(movie.watched_date)}</span
                            >
                        </div>
                    {/if}
                    {#if movie.notes}
                        <div class="info-row">
                            <span class="info-label">Notes:</span>
                            <span class="info-value notes">{movie.notes}</span>
                        </div>
                    {/if}
                </div>
            {:else if getMediaType(media) === "series"}
                {@const series = media as TVSeries}
                <div class="info-group">
                    {#if series.release_year}
                        <div class="info-row">
                            <span class="info-label">Year:</span>
                            <span class="info-value">{series.release_year}</span
                            >
                        </div>
                    {/if}
                    {#if series.status}
                        <div class="info-row">
                            <span class="info-label">Status:</span>
                            <span class="info-value status"
                                >{series.status}</span
                            >
                        </div>
                    {/if}
                    {#if series.total_seasons}
                        <div class="info-row">
                            <span class="info-label">Total Seasons:</span>
                            <span class="info-value"
                                >{series.total_seasons}</span
                            >
                        </div>
                    {/if}
                    {#if series.current_season}
                        <div class="info-row">
                            <span class="info-label">Current Season:</span>
                            <span class="info-value"
                                >{series.current_season}</span
                            >
                        </div>
                    {/if}
                    {#if series.current_episode}
                        <div class="info-row">
                            <span class="info-label">Current Episode:</span>
                            <span class="info-value"
                                >{series.current_episode}</span
                            >
                        </div>
                    {/if}
                    {#if series.rating}
                        <div class="info-row">
                            <span class="info-label">Rating:</span>
                            <span class="info-value rating-value"
                                >★ {series.rating.toFixed(1)}</span
                            >
                        </div>
                    {/if}
                    {#if series.started_date}
                        <div class="info-row">
                            <span class="info-label">Started:</span>
                            <span class="info-value"
                                >{formatDate(series.started_date)}</span
                            >
                        </div>
                    {/if}
                    {#if series.completed_date}
                        <div class="info-row">
                            <span class="info-label">Completed:</span>
                            <span class="info-value"
                                >{formatDate(series.completed_date)}</span
                            >
                        </div>
                    {/if}
                    {#if series.notes}
                        <div class="info-row">
                            <span class="info-label">Notes:</span>
                            <span class="info-value notes">{series.notes}</span>
                        </div>
                    {/if}
                </div>
            {:else}
                {@const book = media as Book}
                <div class="info-group">
                    {#if book.author}
                        <div class="info-row">
                            <span class="info-label">Author:</span>
                            <span class="info-value">{book.author}</span>
                        </div>
                    {/if}
                    {#if book.isbn}
                        <div class="info-row">
                            <span class="info-label">ISBN:</span>
                            <span class="info-value">{book.isbn}</span>
                        </div>
                    {/if}
                    {#if book.genre}
                        <div class="info-row">
                            <span class="info-label">Genre:</span>
                            <span class="info-value">{book.genre}</span>
                        </div>
                    {/if}
                    {#if book.pages}
                        <div class="info-row">
                            <span class="info-label">Pages:</span>
                            <span class="info-value">{book.pages}</span>
                        </div>
                    {/if}
                    {#if book.rating}
                        <div class="info-row">
                            <span class="info-label">Rating:</span>
                            <span class="info-value rating-value"
                                >★ {book.rating.toFixed(1)}</span
                            >
                        </div>
                    {/if}
                    {#if book.started_date}
                        <div class="info-row">
                            <span class="info-label">Started:</span>
                            <span class="info-value"
                                >{formatDate(book.started_date)}</span
                            >
                        </div>
                    {/if}
                    {#if book.completed_date}
                        <div class="info-row">
                            <span class="info-label">Completed:</span>
                            <span class="info-value"
                                >{formatDate(book.completed_date)}</span
                            >
                        </div>
                    {/if}
                    {#if book.notes}
                        <div class="info-row">
                            <span class="info-label">Notes:</span>
                            <span class="info-value notes">{book.notes}</span>
                        </div>
                    {/if}
                </div>
            {/if}
        </div>

        <div class="overlay-actions">
            {#if onEdit}
                <button
                    class="action-btn edit-btn"
                    onclick={(e) => {
                        handleButtonClick(e);
                        onEdit?.(media);
                    }}
                >
                    Edit
                </button>
            {/if}
            {#if onDelete}
                <button
                    class="action-btn delete-btn"
                    onclick={(e) => {
                        handleButtonClick(e);
                        onDelete?.(media);
                    }}
                >
                    Delete
                </button>
            {/if}
        </div>
    </div>
</div>

<style>
    .media-card {
        background: var(--card-bg);
        border: 1px solid var(--card-border);
        border-radius: 12px;
        overflow: hidden;
        transition:
            transform 0.2s ease,
            box-shadow 0.2s ease;
        display: flex;
        flex-direction: column;
        height: 100%;
        position: relative;
        cursor: pointer;
        touch-action: manipulation;
    }

    .media-card:active {
        transform: scale(0.98);
        box-shadow: var(--shadow-lg);
    }

    .media-image-container {
        width: 100%;
        aspect-ratio: 2 / 3;
        background: var(--bg-tertiary);
        overflow: hidden;
        flex-shrink: 0;
        transition: filter 0.3s ease;
    }

    .media-image-container.blurred {
        filter: blur(8px);
    }

    .media-image {
        width: 100%;
        height: 100%;
        object-fit: cover;
    }

    .media-info {
        padding: 12px;
        display: flex;
        flex-direction: column;
        gap: 8px;
        flex-shrink: 0;
        transition:
            visibility 0.2s ease,
            opacity 0.2s ease;
    }

    .media-info.invisible {
        visibility: hidden;
        opacity: 0;
    }

    .title-year {
        flex: 1;
        overflow: hidden;
    }

    .media-title {
        font-size: 14px;
        font-weight: 600;
        color: var(--text-primary);
        margin: 0;
        display: -webkit-box;
        -webkit-line-clamp: 2;
        line-clamp: 2;
        -webkit-box-orient: vertical;
        overflow: hidden;
        line-height: 1.3;
    }

    .year {
        font-size: 12px;
        font-weight: 400;
        color: var(--text-secondary);
        margin-left: 4px;
    }

    .media-rating {
        font-size: 12px;
        color: var(--rating-color);
        font-weight: 500;
        margin: 0;
    }

    .hover-overlay {
        position: absolute;
        inset: 0;
        background: rgba(0, 0, 0, 0.85);
        display: flex;
        flex-direction: column;
        padding: 12px;
        z-index: 10;
        opacity: 0;
        visibility: hidden;
        transition:
            opacity 0.2s ease,
            visibility 0.2s ease;
    }

    .hover-overlay.visible {
        opacity: 1;
        visibility: visible;
    }

    .overlay-content {
        flex: 1;
        display: flex;
        flex-direction: column;
        gap: 4px;
        overflow-y: auto;
        padding-right: 4px;
    }

    .overlay-content::-webkit-scrollbar {
        width: 4px;
    }

    .overlay-content::-webkit-scrollbar-track {
        background: rgba(255, 255, 255, 0.1);
        border-radius: 2px;
    }

    .overlay-content::-webkit-scrollbar-thumb {
        background: rgba(255, 255, 255, 0.3);
        border-radius: 2px;
    }

    .overlay-content::-webkit-scrollbar-thumb:hover {
        background: rgba(255, 255, 255, 0.5);
    }

    .overlay-title {
        font-size: 13px;
        font-weight: 600;
        color: white;
        margin: 0 0 6px 0;
        line-height: 1.3;
    }

    .info-group {
        display: flex;
        flex-direction: column;
        gap: 4px;
    }

    .info-row {
        display: flex;
        gap: 6px;
        font-size: 11px;
        line-height: 1.2;
    }

    .info-label {
        color: rgba(255, 255, 255, 0.7);
        font-weight: 500;
        flex-shrink: 0;
        min-width: 70px;
    }

    .info-value {
        color: rgba(255, 255, 255, 0.9);
        flex: 1;
        overflow: hidden;
        text-overflow: ellipsis;
        display: -webkit-box;
        -webkit-line-clamp: 2;
        line-clamp: 2;
        -webkit-box-orient: vertical;
    }

    .info-value.rating-value {
        color: var(--accent-primary);
        font-weight: 600;
    }

    .info-value.status {
        color: var(--accent-primary);
        font-weight: 500;
    }

    .info-value.notes {
        color: rgba(255, 255, 255, 0.8);
        font-style: italic;
    }

    .overlay-actions {
        display: flex;
        gap: 8px;
        margin-top: 10px;
        flex-shrink: 0;
    }

    .action-btn {
        flex: 1;
        padding: 8px 10px;
        border: none;
        border-radius: 6px;
        font-size: 12px;
        font-weight: 600;
        cursor: pointer;
        transition: background-color 0.2s ease;
        text-transform: uppercase;
        letter-spacing: 0.5px;
    }

    .edit-btn {
        background: var(--btn-edit-bg);
        color: var(--btn-edit-text);
    }

    .edit-btn:active {
        background: var(--btn-edit-hover);
    }

    .delete-btn {
        background: var(--btn-delete-bg);
        color: var(--btn-delete-text);
    }

    .delete-btn:active {
        background: var(--btn-delete-hover);
    }

    @media (min-width: 640px) {
        .media-card:hover {
            box-shadow: var(--shadow-lg);
        }

        .edit-btn:hover {
            background: var(--btn-edit-hover);
        }

        .delete-btn:hover {
            background: var(--btn-delete-hover);
        }
    }
</style>
