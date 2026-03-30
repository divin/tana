/**
 * Tana Media Tracker - Main Application Logic
 * Shared functionality across all pages
 */

// ============================================================================
// Global State
// ============================================================================

const appState = {
  currentPage: null,
  editingId: null,
  editingType: null,
  deletingId: null,
  deletingType: null,
  searchQuery: "",
  filters: {},
  loading: false,
};

// ============================================================================
// Initialization
// ============================================================================

document.addEventListener("DOMContentLoaded", function () {
  console.log("Tana App Initialized");
  initializeEventListeners();
  initializeModals();
});

/**
 * Initialize global event listeners
 */
function initializeEventListeners() {
  // Navigation links
  document.querySelectorAll(".nav-link").forEach((link) => {
    link.addEventListener("click", function (e) {
      e.preventDefault();
      const page = this.dataset.page || this.getAttribute("href");
      navigateTo(page);
    });
  });

  // Close modals on escape key
  document.addEventListener("keydown", function (e) {
    if (e.key === "Escape") {
      document.querySelectorAll("dialog[open]").forEach((dialog) => {
        dialog.close();
      });
    }
  });

  // Close alert on click
  document.addEventListener("click", function (e) {
    if (e.target.classList.contains("alert-close")) {
      e.target.closest(".alert").remove();
    }
  });
}

/**
 * Initialize modal behaviors
 */
function initializeModals() {
  document.querySelectorAll("dialog").forEach((dialog) => {
    // Close on backdrop click
    dialog.addEventListener("click", function (e) {
      if (e.target === this) {
        this.close();
      }
    });

    // Close on escape
    dialog.addEventListener("keydown", function (e) {
      if (e.key === "Escape") {
        this.close();
      }
    });
  });
}

// ============================================================================
// Modal Management
// ============================================================================

/**
 * Open a modal dialog
 * @param {string} modalId - ID of modal to open
 */
function openModal(modalId) {
  const modal = document.getElementById(modalId);
  if (modal) {
    modal.showModal();
  }
}

/**
 * Close a modal dialog
 * @param {string} modalId - ID of modal to close
 */
function closeModal(modalId) {
  const modal = document.getElementById(modalId);
  if (modal) {
    modal.close();
  }
}

/**
 * Open add/edit modal for media
 * @param {string} type - Media type (movie, series, book)
 * @param {object} data - Data to edit (optional)
 */
function openMediaModal(type, data = null) {
  const modalId = `modal-${type}`;
  const modal = document.getElementById(modalId);
  const form = document.getElementById(`form-${type}`);
  const titleEl = document.getElementById(`modal-${type}-title`);

  if (!modal || !form) {
    console.error(`Modal or form not found for type: ${type}`);
    return;
  }

  // Set modal title
  if (data && data.id) {
    titleEl.textContent = `Edit ${type.charAt(0).toUpperCase() + type.slice(1)}`;
    appState.editingId = data.id;
    appState.editingType = type;
    populateForm(form, data);
  } else {
    titleEl.textContent = `Add ${type.charAt(0).toUpperCase() + type.slice(1)}`;
    appState.editingId = null;
    appState.editingType = null;
    clearForm(form);
  }

  openModal(modalId);
}

/**
 * Close media modal
 * @param {string} type - Media type
 */
function closeMediaModal(type) {
  closeModal(`modal-${type}`);
}

// ============================================================================
// Delete Confirmation
// ============================================================================

/**
 * Show delete confirmation dialog
 * @param {string} id - ID of item to delete
 * @param {string} type - Type of item (movie, series, book)
 * @param {string} title - Title/name of item
 */
function showDeleteConfirmation(id, type, title) {
  appState.deletingId = id;
  appState.deletingType = type;

  const dialog = document.getElementById("dialog-confirm-delete");
  const message = document.getElementById("delete-message");
  const confirmBtn = document.getElementById("confirm-delete-btn");

  if (!dialog) return;

  message.textContent = `Are you sure you want to delete "${title}"? This action cannot be undone.`;

  // Clear previous handler
  confirmBtn.onclick = null;

  // Set new handler
  confirmBtn.onclick = async () => {
    await confirmDelete(id, type);
    dialog.close();
  };

  openModal("dialog-confirm-delete");
}

/**
 * Confirm delete action
 * @param {string} id - Item ID
 * @param {string} type - Item type
 */
async function confirmDelete(id, type) {
  const confirmBtn = document.getElementById("confirm-delete-btn");
  loadingStart(confirmBtn);

  try {
    let deletePromise;
    switch (type.toLowerCase()) {
      case "movie":
        deletePromise = apiClient.deleteMovie(id);
        break;
      case "series":
        deletePromise = apiClient.deleteSeries(id);
        break;
      case "book":
        deletePromise = apiClient.deleteBook(id);
        break;
      default:
        throw new Error("Unknown media type");
    }

    await deletePromise;
    showSuccess(
      `${type.charAt(0).toUpperCase() + type.slice(1)} deleted successfully`,
    );

    // Reload current page
    location.reload();
  } catch (error) {
    console.error("Delete error:", error);
    showError(error.message || "Failed to delete item");
  } finally {
    loadingEnd(confirmBtn);
  }
}

// ============================================================================
// Media Card Rendering
// ============================================================================

/**
 * Create a media card HTML element
 * @param {object} media - Media object
 * @param {string} type - Media type
 * @returns {HTMLElement} Card element
 */
function createMediaCard(media, type) {
  const card = document.createElement("div");
  card.className = "media-card";

  const imageUrl = getImageUrl(media);
  const hasImage = imageUrl && imageUrl.trim() !== "";

  const imageHtml = hasImage
    ? `<img src="${escapeHtml(imageUrl)}" alt="${escapeHtml(media.title || "Media")}" onerror="this.parentElement.innerHTML='${getMediaTypeIcon(type)}'">`
    : `<span class="media-card-image-placeholder">${getMediaTypeIcon(type)}</span>`;

  const ratingHtml = media.rating
    ? `<span class="media-card-rating">⭐ ${parseFloat(media.rating).toFixed(1)}</span>`
    : "";

  let metaHtml = "";
  if (type === "movie" && media.year) {
    metaHtml += `<span>${media.year}</span>`;
  } else if (type === "series") {
    if (media.year_started) {
      metaHtml += `<span>${media.year_started}</span>`;
    }
    if (media.current_progress) {
      metaHtml += `<span class="media-card-progress">${escapeHtml(media.current_progress)}</span>`;
    }
  } else if (type === "book" && media.author) {
    metaHtml += `<span>${escapeHtml(media.author)}</span>`;
  }

  card.innerHTML = `
        <div class="media-card-image">
            ${imageHtml}
        </div>
        <div class="media-card-content">
            <h4 class="media-card-title" title="${escapeHtml(media.title || "")}">${escapeHtml(media.title || "Untitled")}</h4>
            <div class="media-card-meta">
                ${metaHtml}
                ${ratingHtml}
            </div>
            ${media.notes ? `<p>${escapeHtml(truncateText(media.notes, 80))}</p>` : ""}
            <div class="media-card-actions">
                <button class="btn-secondary" onclick="editMedia('${media.id}', '${type}')">✏️ Edit</button>
                <button class="btn-danger" onclick="deleteMedia('${media.id}', '${type}', '${escapeHtml(media.title)}')">🗑️ Delete</button>
            </div>
        </div>
    `;

  return card;
}

/**
 * Edit media item
 * @param {string} id - Item ID
 * @param {string} type - Item type
 */
async function editMedia(id, type) {
  try {
    let media;
    switch (type.toLowerCase()) {
      case "movie":
        media = await apiClient.getMovie(id);
        break;
      case "series":
        media = await apiClient.getSeriesById(id);
        break;
      case "book":
        media = await apiClient.getBook(id);
        break;
      default:
        throw new Error("Unknown media type");
    }

    openMediaModal(type, media);
  } catch (error) {
    console.error("Edit error:", error);
    showError("Failed to load item");
  }
}

/**
 * Delete media item
 * @param {string} id - Item ID
 * @param {string} type - Item type
 * @param {string} title - Item title
 */
function deleteMedia(id, type, title) {
  showDeleteConfirmation(id, type, title);
}

// ============================================================================
// Form Handling
// ============================================================================

/**
 * Handle media form submission (create or update)
 * @param {Event} e - Form submit event
 * @param {string} type - Media type
 */
async function handleMediaFormSubmit(e, type) {
  e.preventDefault();

  const form = e.target;
  const button = form.querySelector('button[type="submit"]');

  // Validate form
  const formData = getFormData(form);
  let validation;

  switch (type.toLowerCase()) {
    case "movie":
      validation = validateMovieForm(formData);
      break;
    case "series":
      validation = validateSeriesForm(formData);
      break;
    case "book":
      validation = validateBookForm(formData);
      break;
    default:
      showError("Unknown media type");
      return;
  }

  if (!validation.isValid) {
    setFormErrors(form, validation.errors);
    showError("Please fix the errors in the form");
    return;
  }

  loadingStart(button);

  try {
    // Clean up empty fields
    Object.keys(formData).forEach((key) => {
      if (formData[key] === null || formData[key] === "") {
        delete formData[key];
      }
    });

    if (appState.editingId) {
      // Update existing
      let updatePromise;
      switch (type.toLowerCase()) {
        case "movie":
          updatePromise = apiClient.updateMovie(appState.editingId, formData);
          break;
        case "series":
          updatePromise = apiClient.updateSeries(appState.editingId, formData);
          break;
        case "book":
          updatePromise = apiClient.updateBook(appState.editingId, formData);
          break;
      }

      await updatePromise;
      showSuccess(
        `${type.charAt(0).toUpperCase() + type.slice(1)} updated successfully`,
      );
    } else {
      // Create new
      let createPromise;
      switch (type.toLowerCase()) {
        case "movie":
          createPromise = apiClient.addMovie(formData);
          break;
        case "series":
          createPromise = apiClient.addSeries(formData);
          break;
        case "book":
          createPromise = apiClient.addBook(formData);
          break;
      }

      await createPromise;
      showSuccess(
        `${type.charAt(0).toUpperCase() + type.slice(1)} added successfully`,
      );
    }

    closeMediaModal(type);
    clearForm(form);

    // Reload page data
    if (window.loadPageData) {
      await window.loadPageData();
    } else {
      location.reload();
    }
  } catch (error) {
    console.error("Form submission error:", error);
    showError(error.message || "Failed to save item");
  } finally {
    loadingEnd(button);
  }
}

// ============================================================================
// Search & Filter
// ============================================================================

/**
 * Perform search
 * @param {string} query - Search query
 */
async function performSearch(query) {
  if (!query || query.trim() === "") {
    return [];
  }

  try {
    const results = await apiClient.search(query);
    return results || [];
  } catch (error) {
    console.error("Search error:", error);
    return [];
  }
}

/**
 * Filter array by search query
 * @param {array} items - Items to filter
 * @param {string} query - Search query
 * @param {array} fields - Fields to search in
 * @returns {array} Filtered items
 */
function filterBySearch(items, query, fields = ["title", "author", "notes"]) {
  if (!query || query.trim() === "") {
    return items;
  }

  const q = query.toLowerCase();
  return items.filter((item) =>
    fields.some((field) => {
      const value = item[field];
      return value && value.toString().toLowerCase().includes(q);
    }),
  );
}

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Navigate to a page
 * @param {string} page - Page to navigate to
 */
function navigateTo(page) {
  appState.currentPage = page;
  // Update active nav link
  document.querySelectorAll(".nav-link").forEach((link) => {
    link.classList.remove("active");
  });
  document.querySelector(`[href="${page}"]`)?.classList.add("active");
}

/**
 * Set form field errors
 * @param {HTMLFormElement} form - Form element
 * @param {object} errors - Error object
 */
function setFormErrors(form, errors) {
  // Clear previous errors
  form.querySelectorAll(".form-group.error").forEach((group) => {
    group.classList.remove("error");
  });

  // Set new errors
  Object.entries(errors).forEach(([fieldName, errorMsg]) => {
    const field = form.elements[fieldName];
    if (field) {
      const group = field.closest(".form-group") || field.parentElement;
      if (group) {
        group.classList.add("error");
        let errorElement = group.querySelector(".error-message");
        if (!errorElement) {
          errorElement = document.createElement("small");
          errorElement.className = "error-message";
          group.appendChild(errorElement);
        }
        errorElement.textContent = errorMsg;
      }
    }
  });
}

/**
 * Get image URL from media object
 * @param {object} media - Media object
 * @returns {string} Image URL
 */
function getImageUrl(media) {
  if (media.poster_url && media.poster_url.trim()) return media.poster_url;
  if (media.cover_url && media.cover_url.trim()) return media.cover_url;
  return "";
}

/**
 * Truncate text
 * @param {string} text - Text to truncate
 * @param {number} length - Max length
 * @returns {string} Truncated text
 */
function truncateText(text, length = 100) {
  if (!text) return "";
  if (text.length <= length) return text;
  return text.substring(0, length) + "...";
}

/**
 * Escape HTML
 * @param {string} text - Text to escape
 * @returns {string} Escaped text
 */
function escapeHtml(text) {
  if (!text) return "";
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML;
}
