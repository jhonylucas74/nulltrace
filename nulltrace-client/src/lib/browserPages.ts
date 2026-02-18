/**
 * Built-in pages for the in-app browser. No network – all content is static HTML strings.
 */

const DEFAULT_URL = "ntml.org";

/** Special in-app URL for the history page (rendered by React, not iframe). */
export const BROWSER_HISTORY_URL = "browser://history";

const PAGE_TITLES: Record<string, string> = {
  "search.example": "Search",
  "ntml.org": "NTML",
  "about:blank": "Blank",
  [BROWSER_HISTORY_URL]: "History",
};

/** Demo search page with fictional brand "Goofys" (no real brands). */
const SEARCH_PAGE_HTML = `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Search</title>
  <style>
    * { box-sizing: border-box; }
    html, body {
      margin: 0;
      padding: 0;
      height: 100%;
      min-height: 100vh;
      font-family: Arial, sans-serif;
      background: #fff;
      color: #202124;
    }
    body {
      display: flex;
      flex-direction: column;
      align-items: center;
    }
    .page-wrap {
      flex: 1;
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      width: 100%;
    }
    .logo {
      font-size: 5rem;
      font-weight: 400;
      letter-spacing: -0.2rem;
      margin-bottom: 1.5rem;
      color: #6b4cdf;
    }
    .logo span:nth-child(2) { color: #e85d8a; }
    .logo span:nth-child(3) { color: #f0b429; }
    .logo span:nth-child(4) { color: #6b4cdf; }
    .logo span:nth-child(5) { color: #2e9e6a; }
    .logo span:nth-child(6) { color: #e85d8a; }
    .search-wrap {
      width: 100%;
      max-width: 584px;
      padding: 0 1rem;
    }
    .search-box {
      display: flex;
      align-items: center;
      width: 100%;
      height: 44px;
      padding: 0 1rem;
      margin-bottom: 1.5rem;
      border: 1px solid #dfe1e5;
      border-radius: 24px;
      background: #fff;
    }
    .search-box:hover, .search-box:focus-within {
      box-shadow: 0 1px 6px rgba(32, 33, 36, 0.28);
      border-color: rgba(223, 225, 229, 0);
    }
    .search-box input {
      flex: 1;
      border: none;
      outline: none;
      font-size: 1rem;
      margin: 0 0.75rem;
      color: #202124;
      background: transparent;
    }
    .buttons {
      display: flex;
      gap: 0.75rem;
      justify-content: center;
    }
    .btn {
      padding: 0.5rem 1.25rem;
      font-size: 0.875rem;
      color: #3c4043;
      background: #f8f9fa;
      border: 1px solid #f8f9fa;
      border-radius: 4px;
      cursor: pointer;
    }
    .btn:hover {
      background: #f1f3f4;
      border-color: #f1f3f4;
      box-shadow: 0 1px 1px rgba(0,0,0,0.1);
    }
    .footer {
      margin-top: auto;
      flex-shrink: 0;
      padding: 1rem;
      font-size: 0.8rem;
      color: #70757a;
    }
  </style>
</head>
<body>
  <div class="page-wrap">
    <div class="logo">
      <span>G</span><span>o</span><span>o</span><span>f</span><span>y</span><span>s</span>
    </div>
    <div class="search-wrap">
      <div class="search-box">
        <input type="text" placeholder="Search" aria-label="Search">
      </div>
      <div class="buttons">
        <button type="button" class="btn">Search</button>
        <button type="button" class="btn">I'm Feeling Goofy</button>
      </div>
    </div>
  </div>
  <div class="footer">This is a demo page. No real search. In-app browser only.</div>
</body>
</html>`;

const BLANK_HTML = `<!DOCTYPE html>
<html lang="en">
<head><meta charset="UTF-8"><title>Blank</title></head>
<body></body>
</html>`;

const NOT_FOUND_HTML = `<!DOCTYPE html>
<html lang="en">
<head><meta charset="UTF-8"><title>Page not found</title>
<style>
  body { font-family: Arial,sans-serif; display: flex; align-items: center; justify-content: center; min-height: 100vh; margin: 0; background: #f1f1f1; color: #333; }
  .box { text-align: center; padding: 2rem; }
  h1 { font-size: 1.5rem; margin-bottom: 0.5rem; }
  p { color: #666; }
</style>
</head>
<body>
  <div class="box">
    <h1>Page not found</h1>
    <p>This address is not in the built-in pages.</p>
  </div>
</body>
</html>`;

/** Error page when site can't be reached (connection failed, timeout). */
export const CONNECTION_ERROR_HTML = `<!DOCTYPE html>
<html lang="en">
<head><meta charset="UTF-8"><title>Site can't be reached</title>
<style>
  body { font-family: Arial,sans-serif; display: flex; align-items: center; justify-content: center; min-height: 100vh; margin: 0; background: #f1f1f1; color: #333; }
  .box { text-align: center; padding: 2rem; }
  h1 { font-size: 1.5rem; margin-bottom: 0.5rem; }
  p { color: #666; }
</style>
</head>
<body>
  <div class="box">
    <h1>This site can't be reached</h1>
    <p>Connection failed. The server may be down or unreachable.</p>
  </div>
</body>
</html>`;

/** Error page for HTTP 4xx/5xx responses. */
export function httpErrorHtml(status: number, reason?: string): string {
  const title = status === 404 ? "404 Not Found" : `${status} Error`;
  const msg = reason || (status === 404 ? "The requested page was not found." : "The server encountered an error.");
  return `<!DOCTYPE html>
<html lang="en">
<head><meta charset="UTF-8"><title>${title}</title>
<style>
  body { font-family: Arial,sans-serif; display: flex; align-items: center; justify-content: center; min-height: 100vh; margin: 0; background: #f1f1f1; color: #333; }
  .box { text-align: center; padding: 2rem; }
  h1 { font-size: 1.5rem; margin-bottom: 0.5rem; }
  p { color: #666; }
</style>
</head>
<body>
  <div class="box">
    <h1>${title}</h1>
    <p>${msg}</p>
  </div>
</body>
</html>`;
}

const PAGES: Record<string, string> = {
  "search.example": SEARCH_PAGE_HTML,
  "ntml.org": BLANK_HTML, // Fetched from VM; placeholder until loaded
  "about:blank": BLANK_HTML,
  [BROWSER_HISTORY_URL]: BLANK_HTML, // Rendered by Browser component, not iframe
};

/**
 * Returns the HTML string for the given URL. Unknown URLs get a "Page not found" document.
 */
export function getPageHtml(url: string): string {
  const normalized = normalizeUrl(url);
  return PAGES[normalized] ?? NOT_FOUND_HTML;
}

/**
 * Returns the display title for the given URL.
 */
export function getPageTitle(url: string): string {
  const normalized = normalizeUrl(url);
  return PAGE_TITLES[normalized] ?? "Page not found";
}

/**
 * Normalize URL for lookup (lowercase, trim; empty becomes default).
 */
function normalizeUrl(url: string): string {
  const u = url.trim().toLowerCase();
  return u || DEFAULT_URL;
}

/** Default URL to open when the browser starts. */
export const DEFAULT_BROWSER_URL = DEFAULT_URL;
