<script lang="ts">
    import type { Movie, TVSeries, Book } from "$lib/api/client";
    import MediaCard from "./MediaCard.svelte";

    type Media = Movie | TVSeries | Book;

    interface Props {
        items: Media[];
        onEdit?: (media: Media) => void;
        onDelete?: (media: Media) => void;
        keyFn?: (media: Media) => string | number;
    }

    let { items, onEdit, onDelete, keyFn }: Props = $props();
    let hoveredKey: string | number | null = $state(null);

    function getDefaultKey(media: Media): string {
        const type =
            "watched_date" in media
                ? "movie"
                : "author" in media
                  ? "book"
                  : "series";
        return `${type}-${media.id}`;
    }

    function getKey(media: Media): string | number {
        return keyFn?.(media) ?? getDefaultKey(media);
    }
</script>

<div class="media-grid">
    {#each items as media (getKey(media))}
        <MediaCard
            {media}
            {onEdit}
            {onDelete}
            isHovered={hoveredKey === getKey(media)}
            onHoverChange={(isHovered) => {
                hoveredKey = isHovered ? getKey(media) : null;
            }}
        />
    {/each}
</div>

<style>
    .media-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
        gap: 16px;
    }

    @media (min-width: 640px) {
        .media-grid {
            grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
            gap: 20px;
        }
    }

    @media (min-width: 1024px) {
        .media-grid {
            grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
        }
    }
</style>
