/**
 * Utility Functions for Tana Media Tracker
 * Formatting, validation, and UI helpers
 */

/**
 * Format ISO date string to readable format
 * @param {string} dateString - ISO date string
 * @returns {string} Formatted date
 */
function formatDate(dateString) {
    if (!dateString) return '';
    try {
        const date = new Date(dateString);
        return date.toLocaleDateString('en-US', {
            year: 'numeric',
            month: 'long',
            day: 'numeric',
        });
    } catch (error) {
        return dateString;
    }
}

/**
 * Format time to relative format (e.g., "2 hours ago")
 * @param {string} dateString - ISO date string
 * @returns {string} Relative time
 */
function formatRelativeTime(dateString) {
    if (!dateString) return '';
    try {
        const date = new Date(dateString);
        const now = new Date();
        const seconds = Math.floor((now - date) / 1000);

        let interval = seconds / 31536000;
        if (interval > 1) return Math.floor(interval) + ' years ago';

        interval = seconds / 2592000;
        if (interval > 1) return Math.floor(interval) + ' months ago';

        interval = seconds / 86400;
        if (interval > 1) return Math.floor(interval) + ' days ago';

        interval = seconds / 3600;
        if (interval > 1) return Math.floor(interval) + ' hours ago';

        interval = seconds / 60;
        if (interval > 1) return Math.floor(interval) + ' minutes ago';

        return 'just now';
    } catch (error) {
        return '';
    }
}

/**
 * Format rating as stars or number
 * @param {number} rating - Rating value (0-10)
 * @param {boolean} showStars - Show as stars or number
 * @returns {string} Formatted rating
 */
function formatRating(rating, showStars = false) {
    if (rating === null || rating === undefined || rating === '') {
        return '—';
    }

    const numRating = parseFloat(rating);
    if (isNaN(numRating)) return '—';

    if (showStars) {
        const fullStars = Math.floor(numRating / 2);
        const hasHalfStar = (numRating % 2) >= 1;
        let stars = '⭐'.repeat(fullStars);
        if (hasHalfStar) stars += '✨';
        return stars || '☆☆☆☆☆';
    }

    return numRating.toFixed(1);
}

/**
 * Get emoji icon for media type
 * @param {string} type - Media type (movie, series, book)
 * @returns {string} Emoji icon
 */
function getMediaTypeIcon(type) {
    const icons = {
        movie: '🎬',
        series: '📺',
        book: '📚',
    };
    return icons[type?.toLowerCase()] || '📌';
}

/**
 * Get display name for media type
 * @param {string} type - Media type
 * @returns {string} Display name
 */
function getMediaTypeName(type) {
    const names = {
        movie: 'Movie',
        series: 'TV Series',
        book: 'Book',
    };
    return names[type?.toLowerCase()] || 'Media';
}

/**
 * Validate movie form data
 * @param {object} data - Form data
 * @returns {object} Validation result with errors
 */
function validateMovieForm(data) {
    const errors = {};

    if (!data.title || data.title.trim() === '') {
        errors.title = 'Title is required';
    }

    if (data.year && (isNaN(data.year) || data.year < 1800 || data.year > 2100)) {
        errors.year = 'Enter a valid year between 1800 and 2100';
    }

    if (data.rating && (isNaN(data.rating) || data.rating < 0 || data.rating > 10)) {
        errors.rating = 'Rating must be between 0 and 10';
    }

    if (data.poster_url && !isValidUrl(data.poster_url)) {
        errors.poster_url = 'Enter a valid URL';
    }

    return {
        isValid: Object.keys(errors).length === 0,
        errors,
    };
}

/**
 * Validate series form data
 * @param {object} data - Form data
 * @returns {object} Validation result with errors
 */
function validateSeriesForm(data) {
    const errors = {};

    if (!data.title || data.title.trim() === '') {
        errors.title = 'Title is required';
    }

    if (data.year_started && (isNaN(data.year_started) || data.year_started < 1900 || data.year_started > 2100)) {
        errors.year_started = 'Enter a valid year';
    }

    if (data.current_progress && !/^S\d+E\d+$/i.test(data.current_progress)) {
        errors.current_progress = 'Use format: S01E05';
    }

    if (data.rating && (isNaN(data.rating) || data.rating < 0 || data.rating > 10)) {
        errors.rating = 'Rating must be between 0 and 10';
    }

    if (data.poster_url && !isValidUrl(data.poster_url)) {
        errors.poster_url = 'Enter a valid URL';
    }

    return {
        isValid: Object.keys(errors).length === 0,
        errors,
    };
}

/**
 * Validate book form data
 * @param {object} data - Form data
 * @returns {object} Validation result with errors
 */
function validateBookForm(data) {
    const errors = {};

    if (!data.title || data.title.trim() === '') {
        errors.title = 'Title is required';
    }

    if (!data.author || data.author.trim() === '') {
        errors.author = 'Author is required';
    }

    if (data.year && (isNaN(data.year) || data.year < 1000 || data.year > 2100)) {
        errors.year = 'Enter a valid year';
    }

    if (data.rating && (isNaN(data.rating) || data.rating < 0 || data.rating > 10)) {
        errors.rating = 'Rating must be between 0 and 10';
    }

    if (data.cover_url && !isValidUrl(data.cover_url)) {
        errors.cover_url = 'Enter a valid URL';
    }

    return {
        isValid: Object.keys(errors).length === 0,
        errors,
    };
}

/**
 * Validate URL format
 * @param {string} url - URL to validate
 * @returns {boolean} Is valid URL
 */
function isValidUrl(url) {
    try {
        new URL(url);
        return true;
    } catch {
        return false;
    }
}

/**
 * Show error message as alert
 * @param {string} message - Error message
 * @param {number} duration - Auto-dismiss duration (ms), 0 for permanent
 */
function showError(message, duration = 5000) {
    showAlert(message, 'error', duration);
}

/**
 * Show success message as alert
 * @param {string} message - Success message
 * @param {number} duration - Auto-dismiss duration (ms), default 3000
 */
function showSuccess(message, duration = 3000) {
    showAlert(message, 'success', duration);
}

/**
 * Show warning message as alert
 * @param {string} message - Warning message
 * @param {number} duration - Auto-dismiss duration (ms)
 */
function showWarning(message, duration = 4000) {
    showAlert(message, 'warning', duration);
}

/**
 * Show info message as alert
 * @param {string} message - Info message
 * @param {number} duration - Auto-dismiss duration (ms)
 */
function showInfo(message, duration = 3000) {
    showAlert(message, 'info', duration);
}

/**
 * Show generic alert/toast notification
 * @param {string} message - Alert message
 * @param {string} type - Alert type (success, error, warning, info)
 * @param {number} duration - Auto-dismiss duration (ms), 0 for permanent
 */
function showAlert(message, type = 'info', duration = 3000) {
    const container = document.getElementById('alert-container');
    if (!container) return;

    const alertId = 'alert-' + Date.now();
    const alert = document.createElement('div');
    alert.id = alertId;
    alert.className = `alert ${type}`;

    const icons = {
        success: '✓',
        error: '✕',
        warning: '!',
        info: 'ℹ',
    };

    alert.innerHTML = `
        <div class="alert-content">
            <div class="alert-message">${escapeHtml(message)}</div>
        </div>
        <button class="alert-close" onclick="document.getElementById('${alertId}').remove()">✕</button>
    `;

    container.appendChild(alert);

    if (duration > 0) {
        setTimeout(() => {
            alert.style.animation = 'slideOut 0.3s ease-out';
            setTimeout(() => alert.remove(), 300);
        }, duration);
    }
}

/**
 * Show loading state on element
 * @param {HTMLElement} element - Element to show loading on
 */
function loadingStart(element) {
    if (!element) return;
    element.disabled = true;
    element.dataset.originalText = element.textContent;
    element.innerHTML = '<span class="spinner"></span> Loading...';
}

/**
 * Hide loading state on element
 * @param {HTMLElement} element - Element to hide loading on
 */
function loadingEnd(element) {
    if (!element) return;
    element.disabled = false;
    element.textContent = element.dataset.originalText || element.textContent;
}

/**
 * Clear form fields
 * @param {HTMLFormElement} form - Form to clear
 */
function clearForm(form) {
    if (!form) return;
    form.reset();
    form.querySelectorAll('input, textarea, select').forEach(field => {
        field.value = '';
        field.classList.remove('error');
    });
}

/**
 * Populate form with data
 * @param {HTMLFormElement} form - Form to populate
 * @param {object} data - Data to populate
 */
function populateForm(form, data) {
    if (!form || !data) return;

    Object.keys(data).forEach(key => {
        const field = form.elements[key];
        if (field) {
            if (field.type === 'checkbox') {
                field.checked = Boolean(data[key]);
            } else {
                field.value = data[key] || '';
            }
        }
    });
}

/**
 * Get form data as object
 * @param {HTMLFormElement} form - Form to get data from
 * @returns {object} Form data
 */
function getFormData(form) {
    if (!form) return {};

    const data = {};
    new FormData(form).forEach((value, key) => {
        if (form.elements[key].type === 'number') {
            data[key] = value ? parseFloat(value) : null;
        } else if (form.elements[key].type === 'checkbox') {
            data[key] = form.elements[key].checked;
        } else {
            data[key] = value || null;
        }
    });

    return data;
}

/**
 * Escape HTML special characters
 * @param {string} text - Text to escape
 * @returns {string} Escaped text
 */
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

/**
 * Debounce function for search/filter inputs
 * @param {function} fn - Function to debounce
 * @param {number} delay - Delay in milliseconds
 * @returns {function} Debounced function
 */
function debounce(fn, delay = 300) {
    let timeoutId;
    return function (...args) {
        clearTimeout(timeoutId);
        timeoutId = setTimeout(() => fn.apply(this, args), delay);
    };
}

/**
 * Check if string matches search query
 * @param {string} text - Text to search in
 * @param {string} query - Search query
 * @returns {boolean} Matches query
 */
function matchesSearch(text, query) {
    if (!text || !query) return true;
    return text.toLowerCase().includes(query.toLowerCase());
}

/**
 * Sort array by property
 * @param {array} array - Array to sort
 * @param {string} property - Property to sort by
 * @param {string} direction - 'asc' or 'desc'
 * @returns {array} Sorted array
 */
function sortByProperty(array, property, direction = 'asc') {
    return [...array].sort((a, b) => {
        const aVal = a[property];
        const bVal = b[property];

        if (aVal === null || aVal === undefined) return 1;
        if (bVal === null || bVal === undefined) return -1;

        let comparison = 0;
        if (typeof aVal === 'string') {
            comparison = aVal.localeCompare(bVal);
        } else {
            comparison = aVal < bVal ? -1 : aVal > bVal ? 1 : 0;
        }

        return direction === 'desc' ? -comparison : comparison;
    });
}

/**
 * Generate unique ID
 * @returns {string} Unique ID
 */
function generateId() {
    return 'id-' + Math.random().toString(36).substr(2, 9) + Date.now();
}

/**
 * Wait for specified time
 * @param {number} ms - Milliseconds to wait
 * @returns {Promise} Promise that resolves after delay
 */
function delay(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

/**
 * Parse query parameters from URL
 * @param {string} url - URL to parse (optional, uses window.location.search if not provided)
 * @returns {object} Query parameters
 */
function parseQueryParams(url = window.location.search) {
    const params = {};
    const searchParams = new URLSearchParams(url);
    searchParams.forEach((value, key) => {
        params[key] = value;
    });
    return params;
}
