/**
 * Books Page - Tana Media Tracker
 * Handles book listing, search, filtering by genre/author, and CRUD operations
 */

let booksData = [];
let filteredBooks = [];
let genresSet = new Set();

/**
 * Initialize books page
 */
document.addEventListener('DOMContentLoaded', async function () {
    console.log('Books page initialized');

    // Load books
    await loadBooks();

    // Setup event listeners
    setupBooksEventListeners();
});

/**
 * Setup event listeners for books page
 */
function setupBooksEventListeners() {
    // Add book button
    const addBtn = document.getElementById('btn-add-book');
    const addBtnEmpty = document.getElementById('btn-add-book-empty');
    if (addBtn) addBtn.addEventListener('click', () => openMediaModal('book'));
    if (addBtnEmpty) addBtnEmpty.addEventListener('click', () => openMediaModal('book'));

    // Search input
    const searchInput = document.getElementById('books-search');
    if (searchInput) {
        searchInput.addEventListener('input', debounce(function() {
            filterAndDisplayBooks();
        }, 300));
    }

    // Genre filter dropdown
    const genreFilter = document.getElementById('genre-filter');
    if (genreFilter) {
        genreFilter.addEventListener('change', function() {
            filterAndDisplayBooks();
        });
    }

    // Sort dropdown
    const sortSelect = document.getElementById('sort-by');
    if (sortSelect) {
        sortSelect.addEventListener('change', function() {
            filterAndDisplayBooks();
        });
    }

    // Form submission
    const form = document.getElementById('form-book');
    if (form) {
        form.addEventListener('submit', function(e) {
            handleMediaFormSubmit(e, 'book');
        });
    }
}

/**
 * Load all books from API
 */
async function loadBooks() {
    try {
        const container = document.getElementById('books-container');
        const emptyState = document.getElementById('empty-state');

        if (container) {
            container.innerHTML = `
                <div class="loading-spinner">
                    <div class="spinner"></div>
                    <p>Loading books...</p>
                </div>
            `;
        }

        // Fetch books
        const response = await apiClient.getBooks();
        booksData = response.data || response || [];

        if (!Array.isArray(booksData)) {
            booksData = [];
        }

        // Extract genres and populate genre filter
        populateGenreFilter();

        // Display books
        filterAndDisplayBooks();

    } catch (error) {
        console.error('Error loading books:', error);
        showError('Failed to load books: ' + error.message);

        const container = document.getElementById('books-container');
        if (container) {
            container.innerHTML = `
                <div class="empty-state" style="grid-column: 1/-1;">
                    <div class="empty-icon">❌</div>
                    <h3>Failed to load books</h3>
                    <p>${escapeHtml(error.message)}</p>
                    <button onclick="location.reload()" class="btn-primary">Retry</button>
                </div>
            `;
        }
    }
}

/**
 * Populate genre filter dropdown with unique genres
 */
function populateGenreFilter() {
    const genreFilter = document.getElementById('genre-filter');
    if (!genreFilter) return;

    genresSet.clear();

    // Collect all unique genres
    booksData.forEach(book => {
        if (book.genre && book.genre.trim()) {
            genresSet.add(book.genre.trim());
        }
    });

    // Keep existing options and add genres
    const currentValue = genreFilter.value;
    const options = genreFilter.querySelectorAll('option');
    const baseOptions = [];

    options.forEach(option => {
        if (option.value === '' || option.getAttribute('data-default') === 'true') {
            baseOptions.push(option.cloneNode(true));
        }
    });

    genreFilter.innerHTML = '';
    baseOptions.forEach(option => {
        genreFilter.appendChild(option);
    });

    // Add genre options
    const sortedGenres = Array.from(genresSet).sort();
    sortedGenres.forEach(genre => {
        const option = document.createElement('option');
        option.value = genre;
        option.textContent = genre;
        genreFilter.appendChild(option);
    });

    genreFilter.value = currentValue;
}

/**
 * Filter and display books based on search, genre, and sort
 */
function filterAndDisplayBooks() {
    const container = document.getElementById('books-container');
    const emptyState = document.getElementById('empty-state');
    const searchInput = document.getElementById('books-search');
    const genreFilter = document.getElementById('genre-filter');
    const sortSelect = document.getElementById('sort-by');

    if (!container) return;

    let filtered = [...booksData];

    // Apply search filter
    if (searchInput && searchInput.value.trim()) {
        const query = searchInput.value.toLowerCase();
        filtered = filtered.filter(book =>
            (book.title && book.title.toLowerCase().includes(query)) ||
            (book.author && book.author.toLowerCase().includes(query)) ||
            (book.genre && book.genre.toLowerCase().includes(query))
        );
    }

    // Apply genre filter
    if (genreFilter && genreFilter.value) {
        const selectedGenre = genreFilter.value.trim();
        if (selectedGenre !== '') {
            filtered = filtered.filter(book =>
                book.genre && book.genre.trim() === selectedGenre
            );
        }
    }

    // Apply sorting
    if (sortSelect) {
        const sortBy = sortSelect.value;
        filtered = sortBooks(filtered, sortBy);
    }

    filteredBooks = filtered;

    // Display
    if (filtered.length === 0) {
        container.innerHTML = '';
        if (emptyState) emptyState.style.display = 'block';
    } else {
        if (emptyState) emptyState.style.display = 'none';

        container.innerHTML = '';
        filtered.forEach(book => {
            const card = createMediaCard(book, 'book');
            container.appendChild(card);
        });
    }
}

/**
 * Sort books array
 */
function sortBooks(books, sortBy) {
    const sorted = [...books];

    switch(sortBy) {
        case 'title':
            sorted.sort((a, b) =>
                (a.title || '').localeCompare(b.title || '')
            );
            break;
        case 'author':
            sorted.sort((a, b) =>
                (a.author || '').localeCompare(b.author || '')
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
    await loadBooks();
};
