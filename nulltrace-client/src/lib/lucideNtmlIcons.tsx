/**
 * Resolves Lucide icon names (from NTML Icon component) to SVG markup
 * for injection into the Browser iframe. Uses lucide-react as the single
 * source of truth; no manual path data.
 */

import React from "react";
import { createRoot } from "react-dom/client";
import { flushSync } from "react-dom";
import {
  BookOpen,
  BookText,
  Code,
  FileCode,
  Home,
  Layout,
  Palette,
  Server,
} from "lucide-react";

// Kebab-case name -> Lucide component (only icons used by NTML docs).
const ICON_MAP: Record<string, React.ComponentType<{ size?: number; className?: string }>> = {
  "file-code": FileCode,
  "book-open": BookOpen,
  layout: Layout,
  palette: Palette,
  code: Code,
  server: Server,
  home: Home,
  "book-text": BookText,
};

const svgCache = new Map<string, string>();

function cacheKey(name: string, size: number, className?: string): string {
  return `${name}:${size}:${className ?? ""}`;
}

/**
 * Returns SVG markup for a Lucide icon by name, or empty string if unknown.
 * Uses lucide-react components rendered to static HTML (cached per name/size/class).
 */
export function renderLucideIconToSvg(
  name: string,
  size: number,
  className?: string
): string {
  const IconComponent = ICON_MAP[name];
  if (!IconComponent) return "";

  const key = cacheKey(name, size, className);
  const cached = svgCache.get(key);
  if (cached !== undefined) return cached;

  const div = document.createElement("div");
  document.body.appendChild(div);
  const root = createRoot(div);
  let html = "";
  flushSync(() => {
    root.render(
      React.createElement(IconComponent, {
        size,
        className: className ?? undefined,
      })
    );
    const svg = div.firstElementChild;
    html = svg?.outerHTML ?? "";
  });
  root.unmount();
  div.remove();
  if (html) svgCache.set(key, html);
  return html;
}

export function getSupportedLucideIconNames(): string[] {
  return Object.keys(ICON_MAP);
}
