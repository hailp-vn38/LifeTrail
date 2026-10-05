import type { StyleSpecification } from "maplibre-gl";
import { mapStylePreset, type MapStyleId } from "../map/style-presets";

const DEFAULT_MAPTILER_STYLE_URL =
  "https://api.maptiler.com/maps/hybrid/style.json";

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
  presetId?: MapStyleId;
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
  presetId = "configured",
}: MapStyleConfiguration): string | StyleSpecification {
  const preset = mapStylePreset(presetId);
  if (preset.provider === "VersaTiles" && preset.url) {
    return `${import.meta.env.BASE_URL.replace(/\/$/, "")}${preset.url}`;
  }
  const url = new URL(preset.url || styleUrl || DEFAULT_MAPTILER_STYLE_URL);
  if (!isMapTilerUrl(url)) return url.toString();

  if (mapTilerKey) url.searchParams.set("key", mapTilerKey);
  if (!isConfiguredKey(url.searchParams.get("key"))) return EMPTY_MAP_STYLE;

  return url.toString();
}
