import type { StyleSpecification } from "maplibre-gl";

const DEFAULT_MAPTILER_STYLE_URL =
  "https://api.maptiler.com/maps/streets-v2/style.json";

export const EMPTY_MAP_STYLE: StyleSpecification = {
  version: 8,
  sources: {},
  layers: [
    {
      id: "lifetrail-map-background",
      type: "background",
      paint: { "background-color": "#f1f5f9" },
    },
  ],
};

interface MapStyleConfiguration {
  styleUrl?: string;
  mapTilerKey?: string;
}

function isMapTilerUrl(url: URL): boolean {
  return url.hostname === "maptiler.com" || url.hostname.endsWith(".maptiler.com");
}

function isConfiguredKey(key: string | null): boolean {
  return Boolean(key && key !== "replace-me");
}

export function resolveMapStyle({
  styleUrl,
  mapTilerKey,
}: MapStyleConfiguration): string | StyleSpecification {
  const url = new URL(styleUrl || DEFAULT_MAPTILER_STYLE_URL);
  if (!isMapTilerUrl(url)) return url.toString();

  if (mapTilerKey) url.searchParams.set("key", mapTilerKey);
  if (!isConfiguredKey(url.searchParams.get("key"))) return EMPTY_MAP_STYLE;

  return url.toString();
}
