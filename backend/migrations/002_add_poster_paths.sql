-- Migration 002: Add Poster and Cover Paths
-- Adds poster_path columns to movies and tv_series tables
-- Adds cover_path column to books table

ALTER TABLE movies ADD COLUMN poster_path TEXT;

ALTER TABLE tv_series ADD COLUMN poster_path TEXT;

ALTER TABLE books ADD COLUMN cover_path TEXT;
