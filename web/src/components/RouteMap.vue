<script setup lang="ts">
import type { Feature, LineString, Point } from "geojson";
import { LngLatBounds, Map as MapLibreMap, type GeoJSONSource } from "maplibre-gl";
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import type { DailyView } from "../api/daily-views";
import {
  FollowCamera,
  followOffsetYPx,
  overviewCamera,
} from "../map/route-playback/camera";
import { PlaybackController } from "../map/route-playback/controller";
import { progressCoordinates, routeBounds } from "../map/route-playback/geometry";
import {
  buildPlaybackInput,
  routeDurationMs,
  type PlaybackValidationError,
} from "../map/route-playback/timeline";
import type {
  MapCoordinate,
  PlaybackFrame,
  PlaybackPoint,
  PlaybackState,
} from "../map/route-playback/types";
import RoutePlaybackControls from "./RoutePlaybackControls.vue";
import { initialMapCamera, routeFitOptions } from "./map-camera";
import { initializeWhenMapLoaded } from "./map-lifecycle";
import { resolveMapStyle } from "./map-style";

const SOURCE_FULL = "daily-route-full";
const SOURCE_PROGRESS = "daily-route-progress";
const SOURCE_START = "daily-route-start";
const SOURCE_END = "daily-route-end";
const SOURCE_CURRENT = "daily-route-current";

const LAYER_FULL = "daily-route-full-line";
const LAYER_PROGRESS = "daily-route-progress-line";
const LAYER_START = "daily-route-start-point";
const LAYER_END = "daily-route-end-point";
const LAYER_CURRENT = "daily-route-current-point";

const ISSUE_TEXT: Record<PlaybackValidationError, string> = {
  "not-linestring": "Dữ liệu route không hợp lệ nên không thể phát lại.",
  "too-few-coordinates": "Route chỉ có một điểm nên không thể phát lại.",
  "missing-timestamps": "Thiếu timestamps nên không thể phát lại theo thời gian GPS.",
  "timestamp-count-mismatch": "Timestamps không khớp số điểm GPS nên không thể phát lại.",
  "invalid-timestamp": "Timestamp không hợp lệ nên không thể phát lại.",
  "non-monotonic-timestamps": "Timestamps không theo thứ tự thời gian nên không thể phát lại.",
};

const props = defineProps<{ dailyView: DailyView }>();
const mapElement = ref<HTMLDivElement>();

let map: MapLibreMap | undefined;
let controller: PlaybackController | undefined;
let followCamera: FollowCamera | undefined;
let playbackPoints: PlaybackPoint[] = [];
let lastFrameState: PlaybackState = "idle";

const mapReady = ref(false);
const playbackState = ref<PlaybackState>("idle");
const playbackSpeed = ref(1);
const routeTimeMs = ref(0);
const durationMs = ref(0);
const startTimeMs = ref(0);

const playbackInput = computed(() => {
  const route = props.dailyView.route;
  if (!route) return null;
  return buildPlaybackInput(route);
});

const disabledReason = computed(() => {
  const input = playbackInput.value;
  if (!input || input.ok) return null;
  return ISSUE_TEXT[input.error];
});

function routeCoordinates(): MapCoordinate[] {
  const coordinates = props.dailyView.route?.geometry.coordinates ?? [];
  return coordinates.map(
    ([longitude, latitude]) => [longitude, latitude] as MapCoordinate,
  );
}

function lineFeature(coordinates: MapCoordinate[]): Feature<LineString> {
  return {
    type: "Feature",
    properties: {},
    geometry: { type: "LineString", coordinates },
  };
}

function pointFeature(coordinate: MapCoordinate): Feature<Point> {
  return {
    type: "Feature",
    properties: {},
    geometry: { type: "Point", coordinates: coordinate },
  };
}

function addRouteLayers(map: MapLibreMap) {
  const route = props.dailyView.route;
  const start = props.dailyView.start;
  const end = props.dailyView.end;
  const canPlay = playbackInput.value?.ok === true;

  if (route) {
    map.addSource(SOURCE_FULL, { type: "geojson", data: route });
    map.addLayer({
      id: LAYER_FULL,
      type: "line",
      source: SOURCE_FULL,
      paint: { "line-color": "#2563eb", "line-width": 4, "line-opacity": 0.35 },
    });
  }
  if (canPlay && playbackPoints.length > 0) {
    const startCoordinate = playbackPoints[0].coordinate;
    map.addSource(SOURCE_PROGRESS, {
      type: "geojson",
      data: lineFeature([startCoordinate, startCoordinate]),
    });
    map.addLayer({
      id: LAYER_PROGRESS,
      type: "line",
      source: SOURCE_PROGRESS,
      paint: { "line-color": "#2563eb", "line-width": 5.5, "line-opacity": 1 },
    });
    map.addSource(SOURCE_CURRENT, { type: "geojson", data: pointFeature(startCoordinate) });
    map.addLayer({
      id: LAYER_CURRENT,
      type: "circle",
      source: SOURCE_CURRENT,
      paint: {
        "circle-radius": 8,
        "circle-color": "#f97316",
        "circle-stroke-width": 2.5,
        "circle-stroke-color": "#ffffff",
      },
    });
  }
  if (start) {
    map.addSource(SOURCE_START, { type: "geojson", data: start });
    map.addLayer({
      id: LAYER_START,
      type: "circle",
      source: SOURCE_START,
      paint: {
        "circle-radius": 7,
        "circle-color": "#16a34a",
        "circle-stroke-width": 2,
        "circle-stroke-color": "#ffffff",
      },
    });
  }
  if (end) {
    map.addSource(SOURCE_END, { type: "geojson", data: end });
    map.addLayer({
      id: LAYER_END,
      type: "circle",
      source: SOURCE_END,
      paint: {
        "circle-radius": 7,
        "circle-color": "#dc2626",
        "circle-stroke-width": 2,
        "circle-stroke-color": "#ffffff",
      },
    });
  }
}

function fitRoute(map: MapLibreMap) {
  const coordinates = routeCoordinates();
  if (!coordinates.length && props.dailyView.start) {
    coordinates.push(props.dailyView.start.geometry.coordinates as MapCoordinate);
  }
  if (!coordinates.length && props.dailyView.end) {
    coordinates.push(props.dailyView.end.geometry.coordinates as MapCoordinate);
  }
  if (!coordinates.length) return;
  const bounds = coordinates.reduce(
    (current, coordinate) => current.extend(coordinate),
    new LngLatBounds(coordinates[0], coordinates[0]),
  );
  map.fitBounds(bounds, routeFitOptions);
}

function setSourceData(sourceId: string, data: Feature<LineString> | Feature<Point>) {
  const source = map?.getSource(sourceId) as GeoJSONSource | undefined;
  source?.setData(data);
}

function applyFrame(frame: PlaybackFrame) {
  playbackState.value = frame.state;
  routeTimeMs.value = frame.routeTimeMs;

  if (map) {
    setSourceData(
      SOURCE_PROGRESS,
      lineFeature(progressCoordinates(playbackPoints, frame.vertexIndex, frame.position)),
    );
    setSourceData(SOURCE_CURRENT, pointFeature(frame.position));

    if (frame.state === "playing" && followCamera) {
      if (lastFrameState !== "playing") {
        followCamera.enter(frame.position, frame.bearing);
      } else {
        followCamera.update(frame.position, frame.bearing);
      }
    }
  }
  lastFrameState = frame.state;
}

function handleOverviewReady() {
  if (!map) return;
  const bounds = routeBounds(playbackPoints);
  if (bounds) {
    overviewCamera(map, bounds);
  }
}

function handlePlay() {
  controller?.play();
}

function handlePause() {
  controller?.pause();
}

function handleRestart() {
  controller?.restart();
}

function handleSeek(timeMs: number) {
  controller?.seek(timeMs);
}

function handleSpeedChange(speed: number) {
  playbackSpeed.value = speed;
  controller?.setSpeed(speed);
}

onMounted(() => {
  if (!mapElement.value) return;
  const camera = initialMapCamera({
    routeCoordinates: routeCoordinates(),
    startCoordinate: props.dailyView.start?.geometry.coordinates as MapCoordinate | undefined,
    endCoordinate: props.dailyView.end?.geometry.coordinates as MapCoordinate | undefined,
  });
  map = new MapLibreMap({
    container: mapElement.value,
    style: resolveMapStyle({
      styleUrl: import.meta.env.VITE_MAP_STYLE_URL,
      mapTilerKey: import.meta.env.VITE_MAPTILER_KEY,
    }),
    ...camera,
  });

  const input = playbackInput.value;
  if (input?.ok) {
    playbackPoints = input.points;
    durationMs.value = routeDurationMs(input.points);
    startTimeMs.value = input.points[0].recordedAtMs;
    controller = new PlaybackController({
      points: input.points,
      speed: playbackSpeed.value,
      events: { onFrame: applyFrame, onOverviewReady: handleOverviewReady },
    });
  }

  initializeWhenMapLoaded(map, () => {
    const activeMap = map;
    if (!activeMap) return;
    addRouteLayers(activeMap);
    fitRoute(activeMap);
    if (controller) {
      followCamera = new FollowCamera(activeMap, {
        offsetYPx: followOffsetYPx(activeMap.getContainer().clientHeight),
      });
    }
    mapReady.value = true;
  });
});

onBeforeUnmount(() => {
  controller?.dispose();
  controller = undefined;
  followCamera?.dispose();
  followCamera = undefined;
  playbackPoints = [];
  map?.remove();
  map = undefined;
});
</script>

<template>
  <div class="route-playback">
    <div ref="mapElement" class="route-map" aria-label="Bản đồ Daily Route" />
    <RoutePlaybackControls
      v-if="dailyView.route"
      :state="playbackState"
      :speed="playbackSpeed"
      :route-time-ms="routeTimeMs"
      :duration-ms="durationMs"
      :start-time-ms="startTimeMs"
      :timezone="dailyView.timezone"
      :map-ready="mapReady"
      :disabled-reason="disabledReason"
      @play="handlePlay"
      @pause="handlePause"
      @restart="handleRestart"
      @seek="handleSeek"
      @speed-change="handleSpeedChange"
    />
  </div>
</template>

<style scoped>
.route-map { min-height: 26rem; width: 100%; }
</style>
