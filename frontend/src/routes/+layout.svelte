<script lang="ts">
    import BottomNav from "$lib/components/BottomNav.svelte";
    import "$lib/styles/theme.css";

    let { children } = $props();
</script>

<svelte:head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />

    <meta
        name="description"
        content="A powerful CLI-first media tracker for movies, series, and books"
    />
    <meta name="theme-color" content="#0f172a" />

    <title>Tana - Media Tracker</title>

    <!-- Favicons -->
    <link rel="icon" type="image/x-icon" href="/favicon.ico" />
    <link rel="icon" type="image/png" sizes="32x32" href="/favicon-32x32.png" />
    <link rel="icon" type="image/png" sizes="16x16" href="/favicon-16x16.png" />
    <link rel="apple-touch-icon" href="/apple-touch-icon.png" />

    <!-- Web App Manifest -->
    <link rel="manifest" href="/site.webmanifest" />
</svelte:head>

<div class="app-container">
    <main class="app-content">
        {@render children()}
    </main>
    <BottomNav />
</div>

<style>
    :global {
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }

        html,
        body {
            height: 100%;
        }

        body {
            font-family:
                -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto,
                "Helvetica Neue", Arial, sans-serif;
            background-color: var(--bg-primary);
            color: var(--text-primary);
            line-height: 1.5;
        }

        a {
            color: var(--link-color);
            text-decoration: none;

            &:hover {
                text-decoration: underline;
                color: var(--link-hover);
            }
        }

        button {
            font-family: inherit;
            font-size: inherit;
            cursor: pointer;
        }
    }

    .app-container {
        display: flex;
        flex-direction: column;
        min-height: 100vh;
    }

    .app-content {
        flex: 1;
        overflow-y: auto;
        padding-bottom: 70px;
    }

    @media (min-width: 640px) {
        .app-content {
            padding-bottom: 60px;
        }
    }
</style>
