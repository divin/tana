# 📟 Tana CLI Documentation

## Overview

Tana is a command-line interface for managing your media consumption. All commands follow this basic structure:

```bash
tana [GLOBAL_OPTIONS] <COMMAND> [COMMAND_OPTIONS]
```

## Global Options

These options can be used with any command:

| Option | Short | Description |
|--------|-------|-------------|
| `--debug` | `-d` | Enable debug logging to see detailed information |
| `--db <PATH>` | | Override the default database path |
| `--help` | `-h` | Show help information |
| `--version` | `-V` | Show version information |

### Examples

```bash
# Run a command with debug logging enabled
tana --debug show movies

# Use a custom database location
tana --db /path/to/custom.db add movie

# Get help for a specific command
tana add --help
```

---

## Commands

### 1. `add` - Add New Media

Add a new movie, TV series, or book to your library.

#### Syntax
```bash
tana add <MEDIA_TYPE> [OPTIONS]
```

#### Media Types

##### `add movie`
Add a new movie to your library.

**Options:**
- `--title <TITLE>` - Movie title (required)
- `--year <YEAR>` - Release year
- `--director <DIRECTOR>` - Director name
- `--rating <RATING>` - Your rating (0-10)
- `--watched-date <DATE>` - Date watched (YYYY-MM-DD)
- `--notes <NOTES>` - Additional notes
- `--poster <PATH>` - Path to poster image

**Examples:**
```bash
# Add a movie with basic info
tana add movie --title "The Matrix" --year 1999 --director "Wachowskis" --rating 9.0

# Add a movie with full details
tana add movie \
  --title "Inception" \
  --year 2010 \
  --director "Christopher Nolan" \
  --rating 9.5 \
  --watched-date 2024-01-15 \
  --notes "Mind-bending masterpiece" \
  --poster ~/movies/inception.jpg

# Add a movie with minimal info
tana add movie --title "Dune"
```

##### `add series`
Add a new TV series to your library.

**Options:**
- `--title <TITLE>` - Series title (required)
- `--year <YEAR>` - Release year
- `--status <STATUS>` - Status: `watching`, `completed`, `on-hold`, `dropped`
- `--total-seasons <NUMBER>` - Total number of seasons
- `--current-season <NUMBER>` - Current season you're on
- `--current-episode <NUMBER>` - Current episode you're on
- `--rating <RATING>` - Your rating (0-10)
- `--started-date <DATE>` - Date you started watching (YYYY-MM-DD)
- `--completed-date <DATE>` - Date you completed (YYYY-MM-DD)
- `--notes <NOTES>` - Additional notes
- `--poster <PATH>` - Path to poster image

**Examples:**
```bash
# Add a series you're currently watching
tana add series \
  --title "Breaking Bad" \
  --year 2008 \
  --status watching \
  --total-seasons 5 \
  --current-season 3 \
  --current-episode 7 \
  --rating 9.5 \
  --started-date 2024-01-01

# Add a completed series
tana add series \
  --title "The Office" \
  --year 2005 \
  --status completed \
  --total-seasons 9 \
  --rating 8.5 \
  --completed-date 2024-12-15 \
  --notes "Hilarious mockumentary"
```

##### `add book`
Add a new book to your library.

**Options:**
- `--title <TITLE>` - Book title (required)
- `--author <AUTHOR>` - Author name (required)
- `--isbn <ISBN>` - ISBN number
- `--genre <GENRE>` - Book genre
- `--pages <NUMBER>` - Number of pages
- `--rating <RATING>` - Your rating (0-10)
- `--started-date <DATE>` - Date you started reading (YYYY-MM-DD)
- `--completed-date <DATE>` - Date you finished reading (YYYY-MM-DD)
- `--notes <NOTES>` - Additional notes
- `--cover <PATH>` - Path to cover image

**Examples:**
```bash
# Add a completed book
tana add book \
  --title "1984" \
  --author "George Orwell" \
  --isbn "978-0451524935" \
  --genre "Dystopian Fiction" \
  --pages 328 \
  --rating 9.0 \
  --completed-date 2024-06-15 \
  --notes "Chilling and thought-provoking"

# Add a book you're reading
tana add book \
  --title "The Rust Book" \
  --author "Steve Klabnik and Carol Nichols" \
  --started-date 2024-01-01 \
  --pages 552
```

---

### 2. `edit` - Edit Media Entries

Update information about an existing media entry.

#### Syntax
```bash
tana edit <MEDIA_TYPE> <ID> [OPTIONS]
```

**Options:** Same as the corresponding `add` subcommand.

**Examples:**
```bash
# Update a movie's rating
tana edit movie 1 --rating 8.5

# Update a series' progress
tana edit series 2 --current-season 4 --current-episode 5

# Update a book's completion status
tana edit book 3 --completed-date 2024-12-20
```

---

### 3. `delete` - Delete Media Entries

Remove a media entry from your library.

#### Syntax
```bash
tana delete <MEDIA_TYPE> <ID>
```

**Examples:**
```bash
# Delete a movie
tana delete movie 1

# Delete a TV series
tana delete series 2

# Delete a book
tana delete book 5
```

---

### 4. `show` - Display Media Entries

View media from your library with optional filtering.

#### Syntax
```bash
tana show [SUBCOMMAND] [OPTIONS]
```

#### Subcommands

##### `show movies`
Display all movies.

**Options:**
- `--limit <NUMBER>` - Limit number of results
- `--recent <DAYS>` - Show only movies watched in the last N days
- `--sort <FIELD>` - Sort by field (title, rating, year, date)
- `--format <FORMAT>` - Output format (table, json, csv)

**Examples:**
```bash
# Show all movies
tana show movies

# Show top 10 movies
tana show movies --limit 10

# Show movies watched in last 30 days
tana show movies --recent 30

# Show movies sorted by rating (highest first)
tana show movies --sort rating

# Export as JSON
tana show movies --format json
```

##### `show series`
Display all TV series.

**Options:**
- `--limit <NUMBER>` - Limit number of results
- `--status <STATUS>` - Filter by status (watching, completed, on-hold, dropped)
- `--sort <FIELD>` - Sort by field (title, rating, status)
- `--format <FORMAT>` - Output format (table, json, csv)

**Examples:**
```bash
# Show all series
tana show series

# Show series you're currently watching
tana show series --status watching

# Show completed series sorted by rating
tana show series --status completed --sort rating

# Show top 5 series
tana show series --limit 5
```

##### `show books`
Display all books.

**Options:**
- `--limit <NUMBER>` - Limit number of results
- `--author <NAME>` - Filter by author
- `--genre <GENRE>` - Filter by genre
- `--sort <FIELD>` - Sort by field (title, author, rating, date)
- `--format <FORMAT>` - Output format (table, json, csv)

**Examples:**
```bash
# Show all books
tana show books

# Show books by a specific author
tana show books --author "George Orwell"

# Show science fiction books
tana show books --genre "Science Fiction"

# Show top 10 highest-rated books
tana show books --sort rating --limit 10
```

##### `show` (no subcommand)
Display all media types at once.

**Examples:**
```bash
# Show overview of all media
tana show

# Show recent entries from all types
tana show --recent 7

# Show top 5 of each type
tana show --limit 5
```

---

### 5. `search` - Search Media

Search across all media types for matching entries.

#### Syntax
```bash
tana search <QUERY> [OPTIONS]
```

**Options:**
- `--type <TYPE>` - Search only in specific type (movie, series, book)
- `--case-sensitive` - Make search case-sensitive
- `--limit <NUMBER>` - Limit number of results

**Examples:**
```bash
# Search for "Matrix"
tana search "Matrix"

# Search for "Nolan" only in movies
tana search "Nolan" --type movie

# Search case-sensitively
tana search "The Office" --case-sensitive

# Limit results to 5
tana search "love" --limit 5
```

---

### 6. `stats` - Show Statistics

Display statistics about your media library.

#### Syntax
```bash
tana stats [OPTIONS]
```

**Options:**
- `--type <TYPE>` - Show stats for specific type only (movie, series, book)
- `--year <YEAR>` - Show stats for specific year
- `--format <FORMAT>` - Output format (table, json)

**Examples:**
```bash
# Show overall statistics
tana stats

# Show statistics for movies only
tana stats --type movie

# Show statistics for 2024
tana stats --year 2024

# Show book statistics as JSON
tana stats --type book --format json
```

Statistics include:
- Total count of entries
- Average rating
- Total time spent (estimated for movies/series)
- Most common genres
- Busiest month/year

---

## Date Formats

All dates should be in `YYYY-MM-DD` format:
- `2024-12-25` ✅ Valid
- `12-25-2024` ❌ Invalid
- `2024/12/25` ❌ Invalid

---

## Rating Scale

Ratings are on a scale of 0-10:
- `10` - Masterpiece
- `9` - Excellent
- `8` - Great
- `7` - Good
- `6` - Above Average
- `5` - Average
- `4` - Below Average
- `3` - Poor
- `2` - Very Poor
- `1` - Terrible
- `0` - Haven't rated

---

## Tips & Tricks

### Batch Operations

Add multiple entries at once by creating a script:

```bash
#!/bin/bash
tana add movie --title "Movie 1" --rating 8.0
tana add movie --title "Movie 2" --rating 9.0
tana add movie --title "Movie 3" --rating 7.5
```

### Export Data

Export your library for backup:

```bash
# Export all movies as JSON
tana show movies --format json > movies.json

# Export all books as CSV
tana show books --format csv > books.csv
```

### Alias Frequently Used Commands

```bash
# In your .bashrc or .zshrc
alias tshow='tana show'
alias tadd='tana add'
alias tstats='tana stats'
```

### Database Backups

```bash
# Backup your database
cp ~/.local/share/tana/tana.db ~/.local/share/tana/tana.db.backup

# Restore from backup
cp ~/.local/share/tana/tana.db.backup ~/.local/share/tana/tana.db
```

---

## Error Messages & Solutions

### "Database not found"
The database file doesn't exist at the expected location. It will be created automatically on first run.

### "Permission denied"
Fix permissions on the tana directory:
```bash
chmod 755 ~/.local/share/tana/
```

### "Invalid date format"
Ensure dates are in `YYYY-MM-DD` format.

### "Rating out of range"
Ratings must be between 0 and 10.

---

## Getting Help

```bash
# Get overall help
tana --help

# Get help for a specific command
tana add --help
tana show --help
tana stats --help

# Enable debug mode for troubleshooting
tana --debug <COMMAND>
```
