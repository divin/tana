/**
 * Tana API Client
 * Handles all communication with the backend using Fetch API
 */

const API_BASE_URL = 'http://localhost:8080/api';

class TanaApiClient {
    constructor(baseUrl = API_BASE_URL) {
        this.baseUrl = baseUrl;
        this.headers = {
            'Content-Type': 'application/json',
        };
    }

    /**
     * Base fetch wrapper with error handling
     */
    async request(endpoint, options = {}) {
        const url = `${this.baseUrl}${endpoint}`;

        try {
            const response = await fetch(url, {
                headers: this.headers,
                ...options,
            });

            if (!response.ok) {
                const errorData = await response.json().catch(() => ({}));
                const error = new Error(
                    errorData.message || `HTTP Error: ${response.status}`
                );
                error.status = response.status;
                error.data = errorData;
                throw error;
            }

            if (response.status === 204) {
                return null;
            }

            return await response.json();
        } catch (error) {
            console.error('API Error:', error);
            throw error;
        }
    }

    /**
     * GET request
     */
    async get(endpoint) {
        return this.request(endpoint, {
            method: 'GET',
        });
    }

    /**
     * POST request
     */
    async post(endpoint, data) {
        return this.request(endpoint, {
            method: 'POST',
            body: JSON.stringify(data),
        });
    }

    /**
     * PUT request
     */
    async put(endpoint, data) {
        return this.request(endpoint, {
            method: 'PUT',
            body: JSON.stringify(data),
        });
    }

    /**
     * PATCH request
     */
    async patch(endpoint, data) {
        return this.request(endpoint, {
            method: 'PATCH',
            body: JSON.stringify(data),
        });
    }

    /**
     * DELETE request
     */
    async delete(endpoint) {
        return this.request(endpoint, {
            method: 'DELETE',
        });
    }

    // ====================================================================
    // Movies API Methods
    // ====================================================================

    /**
     * Get all movies
     */
    async getMovies() {
        return this.get('/movies');
    }

    /**
     * Get a single movie by ID
     */
    async getMovie(id) {
        return this.get(`/movies/${id}`);
    }

    /**
     * Create a new movie
     */
    async addMovie(movieData) {
        return this.post('/movies', movieData);
    }

    /**
     * Update an existing movie
     */
    async updateMovie(id, movieData) {
        return this.put(`/movies/${id}`, movieData);
    }

    /**
     * Delete a movie
     */
    async deleteMovie(id) {
        return this.delete(`/movies/${id}`);
    }

    // ====================================================================
    // Series API Methods
    // ====================================================================

    /**
     * Get all series
     */
    async getSeries() {
        return this.get('/series');
    }

    /**
     * Get a single series by ID
     */
    async getSeriesById(id) {
        return this.get(`/series/${id}`);
    }

    /**
     * Create a new series
     */
    async addSeries(seriesData) {
        return this.post('/series', seriesData);
    }

    /**
     * Update an existing series
     */
    async updateSeries(id, seriesData) {
        return this.put(`/series/${id}`, seriesData);
    }

    /**
     * Delete a series
     */
    async deleteSeries(id) {
        return this.delete(`/series/${id}`);
    }

    // ====================================================================
    // Books API Methods
    // ====================================================================

    /**
     * Get all books
     */
    async getBooks() {
        return this.get('/books');
    }

    /**
     * Get a single book by ID
     */
    async getBook(id) {
        return this.get(`/books/${id}`);
    }

    /**
     * Create a new book
     */
    async addBook(bookData) {
        return this.post('/books', bookData);
    }

    /**
     * Update an existing book
     */
    async updateBook(id, bookData) {
        return this.put(`/books/${id}`, bookData);
    }

    /**
     * Delete a book
     */
    async deleteBook(id) {
        return this.delete(`/books/${id}`);
    }

    // ====================================================================
    // Search API Methods
    // ====================================================================

    /**
     * Search across all media types
     */
    async search(query, mediaType = null) {
        const params = new URLSearchParams();
        params.append('q', query);
        if (mediaType) {
            params.append('type', mediaType);
        }
        return this.get(`/search?${params.toString()}`);
    }

    // ====================================================================
    // Statistics API Methods
    // ====================================================================

    /**
     * Get collection statistics
     */
    async getStats() {
        return this.get('/stats');
    }

    /**
     * Get recent items
     */
    async getRecent(limit = 5) {
        return this.get(`/recent?limit=${limit}`);
    }

    // ====================================================================
    // Utility Methods
    // ====================================================================

    /**
     * Set API base URL (useful for testing)
     */
    setBaseUrl(url) {
        this.baseUrl = url;
    }

    /**
     * Get API base URL
     */
    getBaseUrl() {
        return this.baseUrl;
    }

    /**
     * Check API health
     */
    async health() {
        try {
            const response = await fetch(`${this.baseUrl}/health`, {
                method: 'GET',
                headers: this.headers,
            });
            return response.ok;
        } catch (error) {
            return false;
        }
    }
}

// Create singleton instance
const apiClient = new TanaApiClient();

// Export for use in modules
if (typeof module !== 'undefined' && module.exports) {
    module.exports = { TanaApiClient, apiClient };
}
