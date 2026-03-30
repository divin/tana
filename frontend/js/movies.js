/**
 * Movies Page - Tana Media Tracker
 * Handles movie listing, search, filtering, and CRUD operations
 */

let moviesData = [];
let filteredMovies = [];

/**
 * Initialize movies page
 */
document.addEventListener('DOMContentLoaded', async function () {
    console.log('Movies page initialized');

    // Load movies
    await loadMovies();

    // Setup event listeners
    setupMoviesEventListeners();
});

/**
 * Setup event listeners for movies page
 */
function setupMoviesEventListeners() {
    // Add movie button
    const addBtn = document.getElementById('btn-add-movie');
    const addBtnEmpty = document.getElementById('btn-add-movie-empty');
    if (addBtn) addBtn.addEventListener('click', () => openMediaModal('movie'));
    if (addBtnEmpty) addBtnEmpty.addEventListener('click', () => openMediaModal('movie'));

    // Search input
    const searchInput = document.getElementById('movies-search');
    if (searchInput) {
        searchInput.addEventListener('input', debounce(function() {
            filterAndDisplayMovies();
        }, 300));
    }

    // Sort dropdown
    const sortSelect = document.getElementById('sort-by');
    if (sortSelect) {
        sortSelect.addEventListener('change', function() {
            filterAndDisplayMovies();
        });
    }

    // Form submission
    const form = document.getElementById('form-movie');
    if (form) {
        form.addEventListener('submit', function(e) {
            handleMediaFormSubmit(e, 'movie');
        });
    }
}

/**
 * Load all movies from API
 */
async function loadMovies() {
    try {
        const container = document.getElementById('movies-container');
        const emptyState = document.getElementById('empty-state');

        if (container) {
            container.innerHTML = `
                <div class="loading-spinner">
                    <div class="spinner"></div>
                    <p>Loading movies...</p>
                </div>
            `;
        }

        // Fetch movies
        const response = await apiClient.getMovies();
        moviesData = response.data || response || [];

        if (!Array.isArray(moviesData)) {
            moviesData = [];
        }

        // Display movies
        filterAndDisplayMovies();

    } catch (error) {
        console.error('Error loading movies:', error);
        showError('Failed to load movies: ' + error.message);

        const container = document.getElementById('movies-container');
        if (container) {
            container.innerHTML = `
                <div class="empty-state" style="grid-column: 1/-1;">
                    <div class="empty-icon">❌</div>
                    <h3>Failed to load movies</h3>
                    <p>${escapeHtml(error.message)}</p>
                    <button onclick="location.reload()" class="btn-primary">Retry</button>
                </div>
            `;
        }
    }
}

/**
 * Filter and display movies based on search and sort
 */
function filterAndDisplayMovies() {
    const container = document.getElementById('movies-container');
    const emptyState = document.getElementById('empty-state');
    const searchInput = document.getElementById('movies-search');
    const sortSelect = document.getElementById('sort-by');

    if (!container) return;

    let filtered = [...moviesData];

    // Apply search filter
    if (searchInput && searchInput.value.trim()) {
        const query = searchInput.value.toLowerCase();
        filtered = filtered.filter(movie =>
            (movie.title && movie.title.toLowerCase().includes(query)) ||
            (movie.year && movie.year.toString().includes(query)) ||
            (movie.rating && movie.rating.toString().includes(query))
        );
    }

    // Apply sorting
    if (sortSelect) {
        const sortBy = sortSelect.value;
        filtered = sortMovies(filtered, sortBy);
    }

    filteredMovies = filtered;

    // Display
    if (filtered.length === 0) {
        container.innerHTML = '';
        if (emptyState) emptyState.style.display = 'block';
    } else {
        if (emptyState) emptyState.style.display = 'none';

        container.innerHTML = '';
        filtered.forEach(movie => {
            const card = createMediaCard(movie, 'movie');
            container.appendChild(card);
        });
    }
}

/**
 * Sort movies array
 */
function sortMovies(movies, sortBy) {
    const sorted = [...movies];

    switch(sortBy) {
        case 'title':
            sorted.sort((a, b) =>
                (a.title || '').localeCompare(b.title || '')
            );
            break;
        case 'year':
            sorted.sort((a, b) => (b.year || 0) - (a.year || 0));
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
    await loadMovies();
};
