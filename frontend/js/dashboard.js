/**
 * Dashboard Page - Tana Media Tracker
 * Displays collection statistics and recent items
 */

/**
 * Initialize dashboard on page load
 */
document.addEventListener('DOMContentLoaded', async function () {
    console.log('Dashboard initialized');

    // Load and display statistics
    await loadDashboardStats();

    // Load and display recent items
    await loadDashboardRecentItems();
});

/**
 * Load and display collection statistics
 */
async function loadDashboardStats() {
    try {
        const stats = await apiClient.getStats();

        // Update stat cards
        const moviesCount = document.getElementById('movies-count');
        const seriesCount = document.getElementById('series-count');
        const booksCount = document.getElementById('books-count');
        const avgRating = document.getElementById('avg-rating');

        if (moviesCount) {
            moviesCount.textContent = stats.movies_count !== undefined ? stats.movies_count : 0;
        }

        if (seriesCount) {
            seriesCount.textContent = stats.series_count !== undefined ? stats.series_count : 0;
        }

        if (booksCount) {
            booksCount.textContent = stats.books_count !== undefined ? stats.books_count : 0;
        }

        if (avgRating) {
            const rating = stats.avg_rating || 0;
            avgRating.textContent = typeof rating === 'number' ? rating.toFixed(1) : '0.0';
        }

    } catch (error) {
        console.warn('Could not load stats from API, using fallback:', error);
        // Fallback: load all data and calculate stats
        await loadDashboardStatsFallback();
    }
}

/**
 * Fallback method to load stats by fetching all data
 */
async function loadDashboardStatsFallback() {
    try {
        let moviesCount = 0;
        let seriesCount = 0;
        let booksCount = 0;
        let totalRating = 0;
        let ratedCount = 0;

        // Get movie stats
        try {
            const movies = await apiClient.getMovies();
            const moviesList = movies.data || movies || [];
            moviesCount = Array.isArray(moviesList) ? moviesList.length : 0;
            moviesList.forEach(m => {
                if (m.rating) {
                    totalRating += parseFloat(m.rating);
                    ratedCount++;
                }
            });
        } catch (error) {
            console.warn('Could not load movies:', error);
        }

        // Get series stats
        try {
            const series = await apiClient.getSeries();
            const seriesList = series.data || series || [];
            seriesCount = Array.isArray(seriesList) ? seriesList.length : 0;
            seriesList.forEach(s => {
                if (s.rating) {
                    totalRating += parseFloat(s.rating);
                    ratedCount++;
                }
            });
        } catch (error) {
            console.warn('Could not load series:', error);
        }

        // Get books stats
        try {
            const books = await apiClient.getBooks();
            const booksList = books.data || books || [];
            booksCount = Array.isArray(booksList) ? booksList.length : 0;
            booksList.forEach(b => {
                if (b.rating) {
                    totalRating += parseFloat(b.rating);
                    ratedCount++;
                }
            });
        } catch (error) {
            console.warn('Could not load books:', error);
        }

        // Calculate average rating
        const avgRating = ratedCount > 0 ? totalRating / ratedCount : 0;

        // Update UI
        const moviesCountEl = document.getElementById('movies-count');
        const seriesCountEl = document.getElementById('series-count');
        const booksCountEl = document.getElementById('books-count');
        const avgRatingEl = document.getElementById('avg-rating');

        if (moviesCountEl) moviesCountEl.textContent = moviesCount;
        if (seriesCountEl) seriesCountEl.textContent = seriesCount;
        if (booksCountEl) booksCountEl.textContent = booksCount;
        if (avgRatingEl) avgRatingEl.textContent = avgRating.toFixed(1);

    } catch (error) {
        console.error('Fallback stats loading failed:', error);
    }
}

/**
 * Load and display recent items
 */
async function loadDashboardRecentItems() {
    try {
        const container = document.getElementById('recent-items');

        if (!container) return;

        // Show loading state
        container.innerHTML = `
            <div class="loading-spinner" style="grid-column: 1/-1;">
                <div class="spinner"></div>
                <p>Loading recent items...</p>
            </div>
        `;

        // Try to get recent items from API
        let recentItems = [];

        try {
            const response = await apiClient.getRecent(5);
            recentItems = response.data || response || [];
        } catch (error) {
            // Fallback: fetch all media and get most recent
            recentItems = await getRecentItemsFallback();
        }

        // Display items
        if (recentItems.length === 0) {
            container.innerHTML = `
                <div class="empty-state" style="grid-column: 1/-1;">
                    <div class="empty-icon">📭</div>
                    <p>No items in your collection yet.</p>
                    <p><a href="pages/movies.html">Add your first movie →</a></p>
                </div>
            `;
            return;
        }

        // Display media cards
        container.innerHTML = '';
        recentItems.slice(0, 5).forEach(item => {
            try {
                const card = createMediaCard(item, item.type || 'movie');
                container.appendChild(card);
            } catch (error) {
                console.warn('Could not create card for item:', item, error);
            }
        });

    } catch (error) {
        console.error('Error loading recent items:', error);

        const container = document.getElementById('recent-items');
        if (container) {
            container.innerHTML = `
                <div class="empty-state" style="grid-column: 1/-1;">
                    <p>Error loading recent items</p>
                    <button onclick="location.reload()" class="btn-primary">Retry</button>
                </div>
            `;
        }
    }
}

/**
 * Fallback method to get recent items
 */
async function getRecentItemsFallback() {
    try {
        const allItems = [];

        // Get movies
        try {
            const movies = await apiClient.getMovies();
            const moviesList = movies.data || movies || [];
            if (Array.isArray(moviesList)) {
                moviesList.forEach(m => {
                    allItems.push({
                        ...m,
                        type: 'movie',
                        timestamp: new Date(m.created_at || 0).getTime()
                    });
                });
            }
        } catch (error) {
            console.warn('Could not load movies:', error);
        }

        // Get series
        try {
            const series = await apiClient.getSeries();
            const seriesList = series.data || series || [];
            if (Array.isArray(seriesList)) {
                seriesList.forEach(s => {
                    allItems.push({
                        ...s,
                        type: 'series',
                        timestamp: new Date(s.created_at || 0).getTime()
                    });
                });
            }
        } catch (error) {
            console.warn('Could not load series:', error);
        }

        // Get books
        try {
            const books = await apiClient.getBooks();
            const booksList = books.data || books || [];
            if (Array.isArray(booksList)) {
                booksList.forEach(b => {
                    allItems.push({
                        ...b,
                        type: 'book',
                        timestamp: new Date(b.created_at || 0).getTime()
                    });
                });
            }
        } catch (error) {
            console.warn('Could not load books:', error);
        }

        // Sort by timestamp (newest first) and return top 5
        return allItems
            .sort((a, b) => (b.timestamp || 0) - (a.timestamp || 0))
            .slice(0, 5);

    } catch (error) {
        console.error('Fallback recent items loading failed:', error);
        return [];
    }
}
