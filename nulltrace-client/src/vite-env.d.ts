/// <reference types="vite/client" />

declare module "*.module.css" {
  const classes: { readonly [key: string]: string };
  export default classes;
}

declare module "react-simple-maps" {
  import type { FC } from "react";
  export const ComposableMap: FC<Record<string, unknown>>;
  export const Geographies: FC<Record<string, unknown>>;
  export const Geography: FC<Record<string, unknown>>;
  export const Line: FC<Record<string, unknown>>;
  export const Marker: FC<Record<string, unknown>>;
}
