# 📊 Database Schema

Tana uses SQLite as its database backend. The database is stored at `~/.local/share/tana/tana.db` and uses migrations to manage schema changes.

## Overview

The database consists of three main tables to track different media types:
- **movies** - Feature films and movies
- **tv_series** - Television series and shows
- **books** - Books and written works

Each table has a consistent set of metadata fields (ratings, timestamps, notes) and media-specific fields.

---

## Tables

### Movies Table

Stores information about movies you've watched.

```sql
CREATE TABLE movies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    release_year INTEGER,
    director TEXT,
    rating REAL,
    watched_date DATE NOT NULL,
    notes TEXT,
    poster_path TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

| Column | Type | Description |
|--------|------|-------------|
| `id` | INTEGER | Unique identifier (auto-incremented) |
| `title` | TEXT | Movie title (required) |
| `release_year` | INTEGER | Year the movie was released |
| `director` | TEXT | Director's name |
| `rating` | REAL | Your personal rating (0-10) |
| `watched_date` | DATE | Date you watched the movie (required) |
| `notes` | TEXT | Personal notes or comments |
| `poster_path` | TEXT | Path to poster image file |
| `created_at` | TIMESTAMP | Timestamp when entry was created |
| `updated_at` | TIMESTAMP | Timestamp of last update |

**Indexes:**
- `idx_movies_watched_date` - On `watched_date DESC` (for chronological queries)
- `idx_movies_rating` - On `rating DESC` (for sorted by rating)
- `idx_movies_release_year` - On `release_year` (for filtering by year)

---

### TV Series Table

Stores information about television series and shows you're tracking.

```sql
CREATE TABLE tv_series (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    release_year INTEGER,
    status TEXT,
    total_seasons INTEGER,
    current_season INTEGER,
    current_episode INTEGER,
    rating REAL,
    started_date DATE NOT NULL,
    completed_date DATE,
    notes TEXT,
    poster_path TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

| Column | Type | Description |
|--------|------|-------------|
| `id` | INTEGER | Unique identifier (auto-incremented) |
| `title` | TEXT | Series title (required) |
| `release_year` | INTEGER | Year the series started |
| `status` | TEXT | Current status (e.g., "watching", "completed", "dropped") |
| `total_seasons` | INTEGER | Total number of seasons |
| `current_season` | INTEGER | Current season you're watching |
| `current_episode` | INTEGER | Current episode you're on |
| `rating` | REAL | Your personal rating (0-10) |
| `started_date` | DATE | Date you started watching (required) |
| `completed_date` | DATE | Date you finished (if completed) |
| `notes` | TEXT | Personal notes or comments |
| `poster_path` | TEXT | Path to poster image file |
| `created_at` | TIMESTAMP | Timestamp when entry was created |
| `updated_at` | TIMESTAMP | Timestamp of last update |

**Indexes:**
- `idx_tv_series_started_date` - On `started_date DESC` (for chronological queries)
- `idx_tv_series_status` - On `status` (for filtering by status)
- `idx_tv_series_rating` - On `rating DESC` (for sorted by rating)

---

### Books Table

Stores information about books you've read.

```sql
CREATE TABLE books (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    author TEXT NOT NULL,
    isbn TEXT,
    genre TEXT,
    pages INTEGER,
    rating REAL,
    started_date DATE,
    completed_date DATE NOT NULL,
    notes TEXT,
    cover_path TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

| Column | Type | Description |
|--------|------|-------------|
| `id` | INTEGER | Unique identifier (auto-incremented) |
| `title` | TEXT | Book title (required) |
| `author` | TEXT | Author name (required) |
| `isbn` | TEXT | ISBN number (if available) |
| `genre` | TEXT | Genre classification |
| `pages` | INTEGER | Total number of pages |
| `rating` | REAL | Your personal rating (0-10) |
| `started_date` | DATE | Date you started reading |
| `completed_date` | DATE | Date you finished reading (required) |
| `notes` | TEXT | Personal notes or comments |
| `cover_path` | TEXT | Path to cover image file |
| `created_at` | TIMESTAMP | Timestamp when entry was created |
| `updated_at` | TIMESTAMP | Timestamp of last update |

**Indexes:**
- `idx_books_completed_date` - On `completed_date DESC` (for chronological queries)
- `idx_books_author` - On `author` (for filtering by author)
- `idx_books_genre` - On `genre` (for filtering by genre)
- `idx_books_rating` - On `rating DESC` (for sorted by rating)

---

## Data Types

- **INTEGER** - Whole numbers (IDs, page counts, years, seasons, episodes)
- **REAL** - Floating-point numbers (ratings: 0.0-10.0)
- **TEXT** - Text strings (titles, names, notes, paths)
- **DATE** - Dates in YYYY-MM-DD format
- **TIMESTAMP** - Automatic timestamps (created_at, updated_at)

---

## Migrations

Tana uses numbered SQL migration files to manage schema evolution:

### Migration 001: Initial Schema
Creates the three main tables (movies, tv_series, books) with initial columns and indexes.
**File:** `migrations/001_initial_schema.sql`

### Migration 002: Add Poster Paths
Adds image storage support:
- `poster_path` to `movies` table
- `poster_path` to `tv_series` table
- `cover_path` to `books` table

**File:** `migrations/002_add_poster_paths.sql`

---

## Design Patterns

### Timestamps
Every table includes `created_at` and `updated_at` timestamps to track when entries are created and modified. These help with auditing and sorting recent changes.

### Ratings
Ratings are stored as REAL values (0-10 scale). NULL values indicate unrated entries.

### Dates
Specific dates (watched_date, started_date, completed_date) track when media was consumed, enabling historical analysis and "recently watched" features.

### Indexes
Strategic indexes are created on frequently-queried columns:
- Date columns (for chronological sorting)
- Ratings (for "top rated" queries)
- Status/Author/Genre (for filtering)

---

## Example Queries

### Get all movies rated 8 or higher, ordered by date watched
```sql
SELECT title, director, rating, watched_date
FROM movies
WHERE rating >= 8
ORDER BY watched_date DESC;
```

### Get active TV series
```sql
SELECT title, status, current_season, current_episode
FROM tv_series
WHERE status IN ('watching', 'on-hold')
ORDER BY started_date DESC;
```

### Get books by author, with highest ratings first
```sql
SELECT title, author, rating, completed_date
FROM books
WHERE author = 'Stephen King'
ORDER BY rating DESC, completed_date DESC;
```

### Get statistics for completed series
```sql
SELECT 
    title,
    total_seasons,
    AVG(rating) as avg_rating,
    COUNT(*) as entries
FROM tv_series
WHERE status = 'completed'
GROUP BY title;
```

---

## Future Extensions

The schema is designed to be extensible for:
- **Anime/Manga** - Additional media_type enum
- **Games** - Separate table with gameplay metrics
- **Podcasts** - Audio media with episode tracking
- **Tags/Collections** - Many-to-many relationship tables
- **Reviews** - Detailed review content and structured ratings
- **Watchlist** - Planned/to-watch entries
