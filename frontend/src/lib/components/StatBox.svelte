<script lang="ts">
    interface Props {
        label: string;
        value: number | string;
        href?: string;
        isTotal?: boolean;
    }

    let { label, value, href, isTotal = false }: Props = $props();

    function handleClick() {
        if (href) {
            window.location.href = href;
        }
    }
</script>

{#if isTotal}
    <div class="stat-box total">
        <div class="stat-label">{label}</div>
        <div class="stat-value">{value}</div>
    </div>
{:else}
    <button class="stat-box" onclick={handleClick}>
        <div class="stat-label">{label}</div>
        <div class="stat-value">{value}</div>
        <div class="stat-link">→</div>
    </button>
{/if}

<style>
    .stat-box {
        background: var(--card-bg);
        border: 1px solid var(--border-color);
        border-radius: 12px;
        padding: 16px;
        position: relative;
        transition:
            transform 0.2s ease,
            box-shadow 0.2s ease;
        font-family: inherit;
        font-size: inherit;
        text-align: left;
        cursor: pointer;
        color: var(--text-primary);
    }

    .stat-box:active {
        transform: scale(0.98);
    }

    .stat-box.total {
        background: linear-gradient(
            135deg,
            var(--accent-primary) 0%,
            var(--accent-primary-dark) 100%
        );
        color: white;
        border: none;
        padding: 24px 16px;
        cursor: default;
        display: flex;
        flex-direction: column;
        justify-content: center;
        align-items: flex-start;
        min-height: 100px;
        text-align: left;
    }

    .stat-label {
        font-size: 13px;
        font-weight: 500;
        color: inherit;
        opacity: 0.9;
        margin-bottom: 4px;
    }

    .stat-value {
        font-size: 32px;
        font-weight: 700;
        color: inherit;
        line-height: 1.2;
    }

    .stat-box.total .stat-value {
        color: white;
        font-size: 40px;
    }

    .stat-link {
        position: absolute;
        top: 16px;
        right: 16px;
        font-size: 20px;
        color: inherit;
        opacity: 0.7;
    }

    @media (min-width: 640px) {
        .stat-box:hover {
            box-shadow: var(--shadow-md);
        }
    }
</style>
