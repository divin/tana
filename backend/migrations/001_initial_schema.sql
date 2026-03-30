-- Migration 001: Initial Schema
-- Creates the initial tables for movies, TV series, and books

CREATE TABLE IF NOT EXISTS movies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    release_year INTEGER,
    director TEXT,
    rating REAL,
    watched_date DATE NOT NULL,
    notes TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS tv_series (
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
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS books (
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
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for common queries
CREATE INDEX IF NOT EXISTS idx_movies_watched_date ON movies(watched_date DESC);
CREATE INDEX IF NOT EXISTS idx_movies_rating ON movies(rating DESC);
CREATE INDEX IF NOT EXISTS idx_movies_release_year ON movies(release_year);

CREATE INDEX IF NOT EXISTS idx_tv_series_started_date ON tv_series(started_date DESC);
CREATE INDEX IF NOT EXISTS idx_tv_series_status ON tv_series(status);
CREATE INDEX IF NOT EXISTS idx_tv_series_rating ON tv_series(rating DESC);

CREATE INDEX IF NOT EXISTS idx_books_completed_date ON books(completed_date DESC);
CREATE INDEX IF NOT EXISTS idx_books_author ON books(author);
CREATE INDEX IF NOT EXISTS idx_books_genre ON books(genre);
CREATE INDEX IF NOT EXISTS idx_books_rating ON books(rating DESC);
