/**
 * TV Series Page - Tana Media Tracker
 * Handles series listing, search, filtering, and CRUD operations
 */

let seriesData = [];
let filteredSeries = [];

/**
 * Initialize series page
 */
document.addEventListener('DOMContentLoaded', async function () {
    console.log('Series page initialized');

    // Load series
    await loadSeries();

    // Setup event listeners
    setupSeriesEventListeners();
});

/**
 * Setup event listeners for series page
 */
function setupSeriesEventListeners() {
    // Add series button
    const addBtn = document.getElementById('btn-add-series');
    const addBtnEmpty = document.getElementById('btn-add-series-empty');
    if (addBtn) addBtn.addEventListener('click', () => openMediaModal('series'));
    if (addBtnEmpty) addBtnEmpty.addEventListener('click', () => openMediaModal('series'));

    // Search input
    const searchInput = document.getElementById('series-search');
    if (searchInput) {
        searchInput.addEventListener('input', debounce(function() {
            filterAndDisplaySeries();
        }, 300));
    }

    // Sort dropdown
    const sortSelect = document.getElementById('sort-by');
    if (sortSelect) {
        sortSelect.addEventListener('change', function() {
            filterAndDisplaySeries();
        });
    }

    // Form submission
    const form = document.getElementById('form-series');
    if (form) {
        form.addEventListener('submit', function(e) {
            handleMediaFormSubmit(e, 'series');
        });
    }
}

/**
 * Load all series from API
 */
async function loadSeries() {
    try {
        const container = document.getElementById('series-container');
        const emptyState = document.getElementById('empty-state');

        if (container) {
            container.innerHTML = `
                <div class="loading-spinner">
                    <div class="spinner"></div>
                    <p>Loading series...</p>
                </div>
            `;
        }

        // Fetch series
        const response = await apiClient.getSeries();
        seriesData = response.data || response || [];

        if (!Array.isArray(seriesData)) {
            seriesData = [];
        }

        // Display series
        filterAndDisplaySeries();

    } catch (error) {
        console.error('Error loading series:', error);
        showError('Failed to load series: ' + error.message);

        const container = document.getElementById('series-container');
        if (container) {
            container.innerHTML = `
                <div class="empty-state" style="grid-column: 1/-1;">
                    <div class="empty-icon">❌</div>
                    <h3>Failed to load series</h3>
                    <p>${escapeHtml(error.message)}</p>
                    <button onclick="location.reload()" class="btn-primary">Retry</button>
                </div>
            `;
        }
    }
}

/**
 * Filter and display series based on search and sort
 */
function filterAndDisplaySeries() {
    const container = document.getElementById('series-container');
    const emptyState = document.getElementById('empty-state');
    const searchInput = document.getElementById('series-search');
    const sortSelect = document.getElementById('sort-by');

    if (!container) return;

    let filtered = [...seriesData];

    // Apply search filter
    if (searchInput && searchInput.value.trim()) {
        const query = searchInput.value.toLowerCase();
        filtered = filtered.filter(series =>
            (series.title && series.title.toLowerCase().includes(query)) ||
            (series.year_started && series.year_started.toString().includes(query)) ||
            (series.current_progress && series.current_progress.toLowerCase().includes(query))
        );
    }

    // Apply sorting
    if (sortSelect) {
        const sortBy = sortSelect.value;
        filtered = sortSeries(filtered, sortBy);
    }

    filteredSeries = filtered;

    // Display
    if (filtered.length === 0) {
        container.innerHTML = '';
        if (emptyState) emptyState.style.display = 'block';
    } else {
        if (emptyState) emptyState.style.display = 'none';

        container.innerHTML = '';
        filtered.forEach(series => {
            const card = createMediaCard(series, 'series');
            container.appendChild(card);
        });
    }
}

/**
 * Sort series array
 */
function sortSeries(seriesList, sortBy) {
    const sorted = [...seriesList];

    switch(sortBy) {
        case 'title':
            sorted.sort((a, b) =>
                (a.title || '').localeCompare(b.title || '')
            );
            break;
        case 'year':
            sorted.sort((a, b) => (b.year_started || 0) - (a.year_started || 0));
            break;
        case 'rating':
            sorted.sort((a, b) => (b.rating || 0) - (a.rating || 0));
            break;
        case 'recent':
        default:
            sorted.sort((a, b) => {
                const dateA = new Date(a.created_at || 0);
                const dateB = new Date(b.created_at || 0);
                return dateB - dateA;
            });
    }

    return sorted;
}

/**
 * Make loadPageData available globally for form submission
 */
window.loadPageData = async function() {
    await loadSeries();
};
