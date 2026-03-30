/**
 * Search Page - Tana Media Tracker
 * Cross-media search functionality with filtering
 */

let allMovies = [];
let allSeries = [];
let allBooks = [];
let searchResults = { movies: [], series: [], books: [] };
let currentSearchQuery = '';

/**
 * Initialize search page
 */
document.addEventListener('DOMContentLoaded', async function () {
    console.log('Search page initialized');

    // Load all media data
    await loadAllMediaData();

    // Setup event listeners
    setupSearchEventListeners();
});

/**
 * Setup event listeners for search page
 */
function setupSearchEventListeners() {
    // Search input
    const searchInput = document.getElementById('search-input');
    if (searchInput) {
        searchInput.addEventListener('input', debounce(function() {
            performSearchQuery();
        }, 300));

        // Auto-focus search input
        searchInput.focus();
    }

    // Media type filter - "All Types" checkbox
    const allCheckbox = document.getElementById('filter-all');
    if (allCheckbox) {
        allCheckbox.addEventListener('change', function() {
            if (this.checked) {
                document.getElementById('filter-movie').checked = false;
                document.getElementById('filter-series').checked = false;
                document.getElementById('filter-book').checked = false;
            }
            displaySearchResults();
        });
    }

    // Individual type checkboxes
    ['movie', 'series', 'book'].forEach(type => {
        const checkbox = document.getElementById(`filter-${type}`);
        if (checkbox) {
            checkbox.addEventListener('change', function() {
                if (this.checked) {
                    allCheckbox.checked = false;
                }
                displaySearchResults();
            });
        }
    });
}

/**
 * Load all media data from API
 */
async function loadAllMediaData() {
    try {
        // Load movies
        try {
            const moviesResponse = await apiClient.getMovies();
            allMovies = moviesResponse.data || moviesResponse || [];
            if (!Array.isArray(allMovies)) allMovies = [];
        } catch (error) {
            console.warn('Failed to load movies:', error);
            allMovies = [];
        }

        // Load series
        try {
            const seriesResponse = await apiClient.getSeries();
            allSeries = seriesResponse.data || seriesResponse || [];
            if (!Array.isArray(allSeries)) allSeries = [];
        } catch (error) {
            console.warn('Failed to load series:', error);
            allSeries = [];
        }

        // Load books
        try {
            const booksResponse = await apiClient.getBooks();
            allBooks = booksResponse.data || booksResponse || [];
            if (!Array.isArray(allBooks)) allBooks = [];
        } catch (error) {
            console.warn('Failed to load books:', error);
            allBooks = [];
        }

        console.log('Loaded media:', {
            movies: allMovies.length,
            series: allSeries.length,
            books: allBooks.length,
        });

    } catch (error) {
        console.error('Error loading media data:', error);
        showError('Failed to load media data: ' + error.message);
    }
}

/**
 * Perform search query
 */
function performSearchQuery() {
    const searchInput = document.getElementById('search-input');
    if (!searchInput) return;

    currentSearchQuery = searchInput.value.trim();

    if (currentSearchQuery === '') {
        displayEmptySearchState();
        return;
    }

    // Search in all media types
    const query = currentSearchQuery.toLowerCase();

    searchResults = {
        movies: searchInMovies(query),
        series: searchInSeries(query),
        books: searchInBooks(query),
    };

    displaySearchResults();
}

/**
 * Search in movies
 */
function searchInMovies(query) {
    return allMovies.filter(movie => {
        const titleMatch = movie.title && movie.title.toLowerCase().includes(query);
        const yearMatch = movie.year && movie.year.toString().includes(query);
        const notesMatch = movie.notes && movie.notes.toLowerCase().includes(query);
        return titleMatch || yearMatch || notesMatch;
    });
}

/**
 * Search in series
 */
function searchInSeries(query) {
    return allSeries.filter(series => {
        const titleMatch = series.title && series.title.toLowerCase().includes(query);
        const yearMatch = series.year_started && series.year_started.toString().includes(query);
        const progressMatch = series.current_progress && series.current_progress.toLowerCase().includes(query);
        const notesMatch = series.notes && series.notes.toLowerCase().includes(query);
        return titleMatch || yearMatch || progressMatch || notesMatch;
    });
}

/**
 * Search in books
 */
function searchInBooks(query) {
    return allBooks.filter(book => {
        const titleMatch = book.title && book.title.toLowerCase().includes(query);
        const authorMatch = book.author && book.author.toLowerCase().includes(query);
        const genreMatch = book.genre && book.genre.toLowerCase().includes(query);
        const notesMatch = book.notes && book.notes.toLowerCase().includes(query);
        return titleMatch || authorMatch || genreMatch || notesMatch;
    });
}

/**
 * Display search results based on filters
 */
function displaySearchResults() {
    const emptyState = document.getElementById('search-empty');
    const noResults = document.getElementById('search-no-results');

    if (!emptyState || !noResults) return;

    // Hide all results sections
    document.querySelectorAll('.results-section').forEach(section => {
        section.style.display = 'none';
    });

    emptyState.style.display = 'none';
    noResults.style.display = 'none';

    if (!currentSearchQuery) {
        displayEmptySearchState();
        return;
    }

    // Get selected filters
    const selectedTypes = getSelectedMediaTypes();
    let totalResults = 0;

    // Display movies results
    if (selectedTypes.includes('movie') && searchResults.movies && searchResults.movies.length > 0) {
        const moviesSection = document.getElementById('movies-results-section');
        const moviesContainer = document.getElementById('movies-results');

        if (moviesSection && moviesContainer) {
            moviesSection.style.display = 'block';
            moviesContainer.innerHTML = '';
            searchResults.movies.forEach(movie => {
                const card = createMediaCard(movie, 'movie');
                moviesContainer.appendChild(card);
            });
            totalResults += searchResults.movies.length;
        }
    }

    // Display series results
    if (selectedTypes.includes('series') && searchResults.series && searchResults.series.length > 0) {
        const seriesSection = document.getElementById('series-results-section');
        const seriesContainer = document.getElementById('series-results');

        if (seriesSection && seriesContainer) {
            seriesSection.style.display = 'block';
            seriesContainer.innerHTML = '';
            searchResults.series.forEach(series => {
                const card = createMediaCard(series, 'series');
                seriesContainer.appendChild(card);
            });
            totalResults += searchResults.series.length;
        }
    }

    // Display books results
    if (selectedTypes.includes('book') && searchResults.books && searchResults.books.length > 0) {
        const booksSection = document.getElementById('books-results-section');
        const booksContainer = document.getElementById('books-results');

        if (booksSection && booksContainer) {
            booksSection.style.display = 'block';
            booksContainer.innerHTML = '';
            searchResults.books.forEach(book => {
                const card = createMediaCard(book, 'book');
                booksContainer.appendChild(card);
            });
            totalResults += searchResults.books.length;
        }
    }

    // Show no results message if nothing matched
    if (totalResults === 0) {
        noResults.style.display = 'block';
    }
}

/**
 * Display empty search state
 */
function displayEmptySearchState() {
    const emptyState = document.getElementById('search-empty');
    const noResults = document.getElementById('search-no-results');

    // Hide all results sections
    document.querySelectorAll('.results-section').forEach(section => {
        section.style.display = 'none';
    });

    if (emptyState) emptyState.style.display = 'block';
    if (noResults) noResults.style.display = 'none';
}

/**
 * Get selected media types from filter checkboxes
 */
function getSelectedMediaTypes() {
    const allCheckbox = document.getElementById('filter-all');
    const movieCheckbox = document.getElementById('filter-movie');
    const seriesCheckbox = document.getElementById('filter-series');
    const bookCheckbox = document.getElementById('filter-book');

    // If "All Types" is checked, return all types
    if (allCheckbox && allCheckbox.checked) {
        return ['movie', 'series', 'book'];
    }

    const selected = [];
    if (movieCheckbox && movieCheckbox.checked) selected.push('movie');
    if (seriesCheckbox && seriesCheckbox.checked) selected.push('series');
    if (bookCheckbox && bookCheckbox.checked) selected.push('book');

    // Return selected types, or all if none selected
    return selected.length > 0 ? selected : ['movie', 'series', 'book'];
}

/**
 * Make loadPageData available globally
 */
window.loadPageData = async function() {
    await loadAllMediaData();
    performSearchQuery();
};
