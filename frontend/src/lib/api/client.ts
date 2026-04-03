import { dev } from "$app/environment";

// API base URL - dynamically constructed based on current host
function getApiBaseUrl(): string {
  if (typeof window === "undefined") {
    // Server-side rendering
    return dev ? "http://localhost:8080/api" : "/api";
  }

  if (!dev) {
    // Production: use relative path
    return "/api";
  }

  // Development: construct URL based on current hostname
  const hostname = window.location.hostname;
  const apiPort = 8080;
  return `http://${hostname}:${apiPort}/api`;
}

// Type definitions for media objects
export interface Movie {
  id: number;
  title: string;
  release_year?: number;
  director?: string;
  rating?: number;
  watched_date: string;
  notes?: string;
  poster_path?: string;
  created_at: string;
  updated_at: string;
}

export interface TVSeries {
  id: number;
  title: string;
  release_year?: number;
  status?: string;
  total_seasons?: number;
  current_season?: number;
  current_episode?: number;
  rating?: number;
  started_date: string;
  completed_date?: string;
  notes?: string;
  poster_path?: string;
  created_at: string;
  updated_at: string;
}

export interface Book {
  id: number;
  title: string;
  author: string;
  isbn?: string;
  genre?: string;
  pages?: number;
  rating?: number;
  started_date?: string;
  completed_date: string;
  notes?: string;
  cover_path?: string;
  created_at: string;
  updated_at: string;
}

export type Media = Movie | TVSeries | Book;

export interface Stats {
  total_movies: number;
  total_series: number;
  total_books: number;
  total_count: number;
}

export interface RecentMedia {
  movies: Movie[];
  series: TVSeries[];
  books: Book[];
}

// Generic error type
export class ApiError extends Error {
  constructor(
    public status: number,
    message: string,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

class ApiClient {
  private baseUrl: string;

  constructor() {
    this.baseUrl = getApiBaseUrl();
  }

  private async request<T>(
    endpoint: string,
    options: RequestInit = {},
  ): Promise<T> {
    const url = `${this.baseUrl}${endpoint}`;

    try {
      const response = await fetch(url, {
        headers: {
          "Content-Type": "application/json",
          ...options.headers,
        },
        ...options,
      });

      if (!response.ok) {
        const errorText = await response.text();
        throw new ApiError(response.status, errorText || response.statusText);
      }

      // Handle 204 No Content responses (e.g., successful DELETE)
      if (response.status === 204) {
        return undefined as any as T;
      }

      return response.json() as Promise<T>;
    } catch (error) {
      if (error instanceof ApiError) {
        throw error;
      }
      throw new Error(
        `Network error: ${error instanceof Error ? error.message : "Unknown error"}`,
      );
    }
  }

  // Movies CRUD
  async getMovies(): Promise<Movie[]> {
    return this.request("/movies");
  }

  async getMovie(id: number): Promise<Movie> {
    return this.request(`/movies/${id}`);
  }

  async createMovie(
    movie: Omit<Movie, "id" | "created_at" | "updated_at">,
  ): Promise<Movie> {
    return this.request("/movies", {
      method: "POST",
      body: JSON.stringify(movie),
    });
  }

  async updateMovie(id: number, movie: Partial<Movie>): Promise<Movie> {
    return this.request(`/movies/${id}`, {
      method: "PUT",
      body: JSON.stringify(movie),
    });
  }

  async deleteMovie(id: number): Promise<void> {
    await this.request(`/movies/${id}`, { method: "DELETE" });
  }

  // TV Series CRUD
  async getSeries(): Promise<TVSeries[]> {
    return this.request("/series");
  }

  async getTVSeries(id: number): Promise<TVSeries> {
    return this.request(`/series/${id}`);
  }

  async createSeries(
    series: Omit<TVSeries, "id" | "created_at" | "updated_at">,
  ): Promise<TVSeries> {
    return this.request("/series", {
      method: "POST",
      body: JSON.stringify(series),
    });
  }

  async updateSeries(id: number, series: Partial<TVSeries>): Promise<TVSeries> {
    return this.request(`/series/${id}`, {
      method: "PUT",
      body: JSON.stringify(series),
    });
  }

  async deleteSeries(id: number): Promise<void> {
    await this.request(`/series/${id}`, { method: "DELETE" });
  }

  // Books CRUD
  async getBooks(): Promise<Book[]> {
    return this.request("/books");
  }

  async getBook(id: number): Promise<Book> {
    return this.request(`/books/${id}`);
  }

  async createBook(
    book: Omit<Book, "id" | "created_at" | "updated_at">,
  ): Promise<Book> {
    return this.request("/books", {
      method: "POST",
      body: JSON.stringify(book),
    });
  }

  async updateBook(id: number, book: Partial<Book>): Promise<Book> {
    return this.request(`/books/${id}`, {
      method: "PUT",
      body: JSON.stringify(book),
    });
  }

  async deleteBook(id: number): Promise<void> {
    await this.request(`/books/${id}`, { method: "DELETE" });
  }

  // Stats and Search
  async getStats(): Promise<Stats> {
    return this.request("/stats");
  }

  async search(query: string): Promise<RecentMedia> {
    return this.request(`/search?q=${encodeURIComponent(query)}`);
  }

  // Dashboard helpers
  async getRecentlyAdded(limit: number = 5): Promise<RecentMedia> {
    const [movies, series, books] = await Promise.all([
      this.getMovies(),
      this.getSeries(),
      this.getBooks(),
    ]);

    const sortByCreatedAt = (a: any, b: any) => {
      return (
        new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
      );
    };

    return {
      movies: movies.sort(sortByCreatedAt).slice(0, limit),
      series: series.sort(sortByCreatedAt).slice(0, limit),
      books: books.sort(sortByCreatedAt).slice(0, limit),
    };
  }
}

// Export singleton instance
export const apiClient = new ApiClient();
