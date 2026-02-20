/**
 * Resolves Lucide icon names (from NTML Icon component) to SVG markup
 * for injection into the Browser iframe. Uses lucide-react dynamicIconImports
 * so all Lucide icons work without manual registration.
 */

import React from "react";
import { createRoot } from "react-dom/client";
import { flushSync } from "react-dom";
import dynamicIconImports from "lucide-react/dynamicIconImports";

const svgCache = new Map<string, string>();

function cacheKey(name: string, size: number, className?: string): string {
  return `${name}:${size}:${className ?? ""}`;
}

/**
 * Returns SVG markup for a Lucide icon by name (async).
 * Uses lucide-react dynamicIconImports - supports all Lucide icons by kebab-case name.
 */
export async function renderLucideIconToSvgAsync(
  name: string,
  size: number,
  className?: string
): Promise<string> {
  const loader = dynamicIconImports[name as keyof typeof dynamicIconImports];
  if (!loader) return "";

  const key = cacheKey(name, size, className);
  const cached = svgCache.get(key);
  if (cached !== undefined) return cached;

  const mod = await loader();
  const IconComponent = mod.default;
  if (!IconComponent) return "";

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
  return Object.keys(dynamicIconImports);
}
