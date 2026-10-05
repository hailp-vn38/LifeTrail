<script setup lang="ts">
import { LngLatBounds, Map as MapLibreMap } from "maplibre-gl";
import { onBeforeUnmount, onMounted, ref } from "vue";
import type { DailyView } from "../api/daily-views";

const props = defineProps<{ dailyView: DailyView }>();
const mapElement = ref<HTMLDivElement>();
let map: MapLibreMap | undefined;

function mapStyleUrl(): string {
  const configuredUrl = import.meta.env.VITE_MAP_STYLE_URL;
  const key = import.meta.env.VITE_MAPTILER_KEY;
  const url = new URL(
    configuredUrl ?? "https://api.maptiler.com/maps/streets-v2/style.json",
  );
  if (key && url.hostname.endsWith("maptiler.com")) url.searchParams.set("key", key);
  return url.toString();
}

function routeCoordinates(): [number, number][] {
  return props.dailyView.route?.geometry.coordinates.map(([longitude, latitude]) => [
    longitude,
    latitude,
  ]) ?? [];
}

function addRouteLayers(map: MapLibreMap) {
  const route = props.dailyView.route;
  const start = props.dailyView.start;
  const end = props.dailyView.end;
  if (route) {
    map.addSource("daily-route", { type: "geojson", data: route });
    map.addLayer({
      id: "daily-route-line",
      type: "line",
      source: "daily-route",
      paint: { "line-color": "#2563eb", "line-width": 5 },
    });
  }
  if (!start || !end) return;

  for (const [name, point, color] of [
    ["daily-route-start", start, "#16a34a"],
    ["daily-route-end", end, "#dc2626"],
  ] as const) {
    map.addSource(name, { type: "geojson", data: point });
    map.addLayer({
      id: `${name}-point`,
      type: "circle",
      source: name,
      paint: { "circle-radius": 7, "circle-color": color, "circle-stroke-width": 2, "circle-stroke-color": "#ffffff" },
    });
  }
}

function fitRoute(map: MapLibreMap) {
  const coordinates = routeCoordinates();
  if (!coordinates.length && props.dailyView.start) {
    coordinates.push(props.dailyView.start.geometry.coordinates as [number, number]);
  }
  if (!coordinates.length && props.dailyView.end) {
    coordinates.push(props.dailyView.end.geometry.coordinates as [number, number]);
  }
  if (!coordinates.length) return;
  const bounds = coordinates.reduce(
    (current, coordinate) => current.extend(coordinate),
    new LngLatBounds(coordinates[0], coordinates[0]),
  );
  map.fitBounds(bounds, { padding: 48, maxZoom: 15 });
}

onMounted(() => {
  if (!mapElement.value) return;
  map = new MapLibreMap({ container: mapElement.value, style: mapStyleUrl() });
  map.on("load", () => {
    const activeMap = map;
    if (!activeMap) return;
    addRouteLayers(activeMap);
    fitRoute(activeMap);
  });
});

onBeforeUnmount(() => map?.remove());
</script>

<template>
  <div ref="mapElement" class="route-map" aria-label="Bản đồ Daily Route" />
</template>

<style scoped>
.route-map { min-height: 26rem; width: 100%; }
</style>
