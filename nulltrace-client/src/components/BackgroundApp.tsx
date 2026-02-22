import { useState, useCallback, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Search } from "lucide-react";
import { useWallpaper } from "../contexts/WallpaperContext";
import styles from "./BackgroundApp.module.css";

const PEXELS_BASE = "https://api.pexels.com/v1";
const DEFAULT_QUERY = "nature landscape";

interface PexelsPhoto {
  id: number;
  width: number;
  height: number;
  url: string;
  photographer: string;
  photographer_url: string;
  src: {
    original: string;
    large2x: string;
    landscape: string;
    medium: string;
    small: string;
  };
  alt: string | null;
}

interface PexelsSearchResponse {
  photos: PexelsPhoto[];
  page: number;
  per_page: number;
  total_results: number;
  next_page?: string;
}

function getApiKey(): string | undefined {
  const key = import.meta.env.VITE_PEXEL_API;
  return typeof key === "string" && key.length > 0 ? key : undefined;
}

async function searchPexels(
  query: string,
  page: number,
  apiKey: string
): Promise<PexelsSearchResponse> {
  const params = new URLSearchParams({
    query,
    orientation: "landscape",
    per_page: "20",
    page: String(page),
  });
  const res = await fetch(`${PEXELS_BASE}/search?${params}`, {
    headers: { Authorization: apiKey },
  });
  if (!res.ok) throw new Error(`Pexels API error: ${res.status}`);
  return res.json();
}

export default function BackgroundApp() {
  const { t } = useTranslation("background");
  const { wallpaperUrl, setWallpaper, gridEnabled, setGridEnabled } = useWallpaper();
  const [searchQuery, setSearchQuery] = useState("");
  const [inputValue, setInputValue] = useState("");
  const [photos, setPhotos] = useState<PexelsPhoto[]>([]);
  const [page, setPage] = useState(1);
  const [hasNext, setHasNext] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const apiKey = getApiKey();

  const doSearch = useCallback(
    async (query: string, pageNum: number = 1) => {
      if (!apiKey) {
        setError(t("api_error"));
        return;
      }
      setError(null);
      setLoading(true);
      try {
        const data = await searchPexels(query, pageNum, apiKey);
        if (pageNum === 1) {
          setPhotos(data.photos);
        } else {
          setPhotos((prev) => [...prev, ...data.photos]);
        }
        setPage(data.page);
        setHasNext(!!data.next_page);
      } catch (e) {
        setError(e instanceof Error ? e.message : t("error_search_failed"));
        if (pageNum === 1) setPhotos([]);
      } finally {
        setLoading(false);
      }
    },
    [apiKey, t]
  );

  useEffect(() => {
    if (!apiKey) return;
    doSearch(DEFAULT_QUERY, 1);
    setSearchQuery(DEFAULT_QUERY);
  }, [apiKey]);

  const handleSearchSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const q = inputValue.trim() || DEFAULT_QUERY;
    setSearchQuery(q);
    doSearch(q, 1);
  };

  const handleLoadMore = () => {
    doSearch(searchQuery, page + 1);
  };

  const wallpaperUrlForPhoto = (photo: PexelsPhoto) =>
    photo.src.large2x || photo.src.original;

  return (
    <div className={styles.app}>
      <div className={styles.main}>
        <div className={styles.content}>
          <div className={styles.sectionHeader}>
            <h2 className={styles.sectionTitle}>{t("section_title")}</h2>
          </div>

          <button
            type="button"
            className={`${styles.noneCard} ${wallpaperUrl === null ? styles.noneCardSelected : ""}`}
            onClick={() => setWallpaper(null)}
          >
            <span>
              <span className={styles.noneLabel}>{t("none_label")}</span>
              <div className={styles.noneHint}>{t("none_hint")}</div>
            </span>
          </button>

          <label className={styles.gridOption}>
            <input
              type="checkbox"
              checked={gridEnabled}
              onChange={(e) => setGridEnabled(e.target.checked)}
              aria-label={t("grid_label")}
            />
            <span className={styles.gridOptionLabel}>{t("grid_label")}</span>
          </label>

          {!apiKey && (
            <div className={styles.apiError}>
              {t("api_error")}
            </div>
          )}

          {apiKey && (
            <>
              <form className={styles.searchRow} onSubmit={handleSearchSubmit}>
                <div className={styles.searchWrap}>
                  <Search size={18} className={styles.searchIcon} aria-hidden />
                  <input
                    type="text"
                    className={styles.searchInput}
                    placeholder={t("search_placeholder")}
                    value={inputValue}
                    onChange={(e) => setInputValue(e.target.value)}
                    aria-label={t("search_aria")}
                  />
                </div>
                <button type="submit" className={styles.searchBtn} disabled={loading}>
                  {t("search_btn")}
                </button>
              </form>

              {error && <div className={styles.apiError}>{error}</div>}
              {loading && photos.length === 0 && (
                <div className={styles.loading}>{t("loading")}</div>
              )}

              {!loading && photos.length === 0 && searchQuery && !error && (
                <div className={styles.emptyState}>{t("empty_state")}</div>
              )}

              {photos.length > 0 && (
                <>
                  <div className={styles.grid}>
                    {photos.map((photo, index) => {
                      const isSelected =
                        wallpaperUrl !== null &&
                        wallpaperUrl === wallpaperUrlForPhoto(photo);
                      return (
                        <button
                          key={photo.id}
                          type="button"
                          className={`${styles.photoCard} ${isSelected ? styles.photoCardSelected : ""}`}
                          style={{ animationDelay: `${index * 40}ms` }}
                          onClick={() => setWallpaper(wallpaperUrlForPhoto(photo))}
                        >
                          <img
                            src={photo.src.medium}
                            alt={photo.alt ?? t("wallpaper_alt")}
                            className={styles.photoImg}
                            loading="lazy"
                          />
                        </button>
                      );
                    })}
                  </div>
                  {hasNext && (
                    <button
                      type="button"
                      className={styles.nextBtn}
                      onClick={handleLoadMore}
                      disabled={loading}
                    >
                      {t("load_more")}
                    </button>
                  )}
                </>
              )}
            </>
          )}

        </div>
      </div>
    </div>
  );
}
