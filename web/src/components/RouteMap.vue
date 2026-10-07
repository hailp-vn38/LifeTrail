<script setup lang="ts">
import type { Feature, FeatureCollection, LineString, Point } from "geojson";
import {
  LngLatBounds,
  Map as MapLibreMap,
  NavigationControl,
  type ExpressionSpecification,
  type GeoJSONSource,
} from "maplibre-gl";
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { ISSUE_TEXT, PART_ISSUE_TEXT, INTERRUPTION_TEXT } from "../map/route-playback/messages";
import { prepareDailyLayers } from "../map/daily-layers";
import { dayClockBounds } from "../map/route-playback/day-clock";
import { progressFeatures } from "../map/route-playback/progress";
import { buildTimelineEvents } from "../features/timeline/events";
import { addEventDots, bindEventDots, revealEventDots, highlightEventDot } from "../map/event-dots";
import type { DailyView } from "../api/queries/daily-view.query";
import {
  FollowCamera,
  followOffsetYPx,
  overviewCamera,
  prefersReducedMotion,
} from "../map/route-playback/camera";
import { addStopLayers, bindStopSelection, focusStop, highlightStop } from "../map/stops";
import {
  addRoutePartLayers,
  bindRoutePartSelection,
  focusTripPart,
  highlightRoutePart,
} from "../map/route-parts";
import { addBuildings } from "../map/buildings";
import { mapStylePreset } from "../map/style-presets";
import { PlaybackInteraction } from "../map/route-playback/interaction";
import { followTarget } from "../map/route-playback/target";
import {
  addRouteLayers,
  lineFeature,
  puckFeatures,
  SOURCE_PROGRESS,
  SOURCE_CURRENT,
  LAYER_START,
  LAYER_END,
} from "../map/route-playback/layers";
import { PlaybackController } from "../map/route-playback/controller";
import {
  progressCoordinates,
  routeBounds,
} from "../map/route-playback/geometry";
import {
  buildPlaybackInput,
  type PlaybackValidationError,
} from "../map/route-playback/timeline";
import { buildRoutePartPlaybackInput } from "../map/route-playback/parts";
import type {
  MapCoordinate,
  PlaybackBreak,
  PlaybackFrame,
  PlaybackPoint,
  PlaybackState,
} from "../map/route-playback/types";
import PlaybackBar from "../features/playback/components/PlaybackBar.vue";
import type { PlaybackSpeed } from "../features/playback/playback.types";
import { evidenceHoles, gapActivities, stopActivities } from "../features/activity/model";
import { useMapStore } from "../stores/map.store";
import { useMapPreferencesStore } from "../stores/map-preferences.store";
import { usePlaybackStore } from "../stores/playback.store";
import { initialMapCamera, routeFitOptions } from "./map-camera";
import { initializeWhenMapLoaded } from "./map-lifecycle";
import { resolveMapStyle } from "./map-style";

const props = defineProps<{ dailyView: DailyView; dayClock?: boolean }>();
const mapElement = ref<HTMLDivElement>();

const mapStore = useMapStore();
const mapPreferences = useMapPreferencesStore();
const playbackStore = usePlaybackStore();

let map: MapLibreMap | undefined;
let controller: PlaybackController | undefined;
let followCamera: FollowCamera | undefined;
let playbackInteraction: PlaybackInteraction | undefined;
let playbackPoints: PlaybackPoint[] = [];
let lastFrameState: PlaybackState = "idle";
let lastFrame: PlaybackFrame | undefined;
let lastFrameTimeMs = 0;
let eventDots: ReturnType<typeof bindEventDots> | undefined;
let resizeObserver: ResizeObserver | undefined;
const timelineEvents = computed(() => buildTimelineEvents(props.dailyView));

const mapReady = ref(false);
const playbackState = ref<PlaybackState>("idle");
const playbackSpeed = ref(1);
const routeTimeMs = ref(0);
const durationMs = ref(0);
const startTimeMs = ref(0);
const interruption = ref<PlaybackBreak>();

const playbackInput = computed(() => {
  const parts = props.dailyView.route_parts ?? [];
  if (parts.length > 0) {
    return buildRoutePartPlaybackInput(parts, gapActivities(props.dailyView), evidenceHoles(props.dailyView));
  }
  const route = props.dailyView.route;
  if (!route) return null;
  return buildPlaybackInput(route);
});

const hasPlaybackSource = computed(() => Boolean(props.dailyView.route || props.dailyView.route_parts?.length));

const disabledReason = computed(() => {
  const input = playbackInput.value;
  if (!input || input.ok) return null;
  if (input.error === "invalid-route-parts" || input.error === "too-few-observations") {
    return PART_ISSUE_TEXT[input.error];
  }
  return ISSUE_TEXT[input.error as PlaybackValidationError];
});

function routeCoordinates(): MapCoordinate[] {
  const coordinates = props.dailyView.route?.geometry.coordinates ?? [];
  return coordinates.map(
    ([longitude, latitude]) => [longitude, latitude] as MapCoordinate,
  );
}

function startCoordinate(): MapCoordinate | undefined {
  const coordinates = props.dailyView.start?.geometry.coordinates;
  return Array.isArray(coordinates) ? (coordinates as MapCoordinate) : undefined;
}

function endCoordinate(): MapCoordinate | undefined {
  const coordinates = props.dailyView.end?.geometry.coordinates;
  return Array.isArray(coordinates) ? (coordinates as MapCoordinate) : undefined;
}

function selectionRadiusExpression(selectedId: string | null): ExpressionSpecification {
  return ["case", ["==", ["get", "eventId"], selectedId ?? "__none__"], 11, 7];
}

function applySelectionHighlight(): void {
  if (!map) return;
  const selectedId = mapStore.selectedEventId;
  highlightStop(map, selectedId);
  if (props.dayClock) highlightEventDot(map, selectedId);
  highlightRoutePart(map, selectedId);
  if (props.dayClock && controller) {
    map.setPaintProperty("trip-route-parts", "line-color", "#94a3b8");
    map.setPaintProperty("trip-route-parts", "line-opacity", 0.6);
  }
  if (props.dailyView.start) {
    map.setPaintProperty(LAYER_START, "circle-radius", selectionRadiusExpression(selectedId));
  }
  if (props.dailyView.end) {
    map.setPaintProperty(LAYER_END, "circle-radius", selectionRadiusExpression(selectedId));
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
  if (!coordinates.length) {
    // Processed activity: fit the published Route Parts, then any Stop centers.
    for (const part of props.dailyView.route_parts ?? []) {
      coordinates.push(...(part.geometry.coordinates as MapCoordinate[]));
    }
  }
  if (!coordinates.length) {
    for (const stop of stopActivities(props.dailyView)) coordinates.push([stop.center[0], stop.center[1]]);
  }
  if (!coordinates.length) return;
  const bounds = coordinates.reduce(
    (current, coordinate) => current.extend(coordinate),
    new LngLatBounds(coordinates[0], coordinates[0]),
  );
  map.fitBounds(bounds, { ...routeFitOptions, pitch: mapStylePreset(mapPreferences.styleId).pitch });
}

function setSourceData(
  sourceId: string,
  data: Feature<LineString> | Feature<Point> | FeatureCollection,
) {
  const source = map?.getSource(sourceId) as GeoJSONSource | undefined;
  source?.setData(data);
}

function applyFrame(frame: PlaybackFrame) {
  playbackState.value = frame.state;
  routeTimeMs.value = frame.routeTimeMs;
  interruption.value = frame.interruption;
  lastFrame = frame;
  playbackStore.setFrame(frame.state, frame.routeTimeMs);

  if (map && mapReady.value) {
    setSourceData(
      SOURCE_PROGRESS,
      props.dayClock ? progressFeatures(playbackPoints, frame, startTimeMs.value + frame.routeTimeMs) : lineFeature(progressCoordinates(playbackPoints, frame.vertexIndex, frame.position)),
    );
    const cursor = startTimeMs.value + frame.routeTimeMs;
    setSourceData(SOURCE_CURRENT, props.dayClock && cursor < playbackPoints[0].recordedAtMs ? { type: "FeatureCollection", features: [] } : puckFeatures(frame.position, frame.bearing));
    if (props.dayClock) { revealEventDots(map, cursor); eventDots?.reveal(cursor); }

    if (followCamera && playbackStore.cameraFollow && (!props.dayClock || cursor >= playbackPoints[0].recordedAtMs)) {
      // Course-Up Follow Camera with Look-Ahead: the camera aims at a point
      // ahead on the route while the puck stays on the current GPS fix.
      const target = followTarget(playbackPoints, frame);
      if (frame.state === "playing") {
        if (lastFrameState !== "playing") {
          followCamera.enter(target);
        } else {
          followCamera.update(target);
        }
      } else if (frame.state === "paused" && frame.routeTimeMs === lastFrameTimeMs) {
        followCamera.stop();
      } else {
        // Seek, restart and the final frame bypass playback throttling.
        followCamera.enter(target);
      }
    }
  }
  if (frame.state === "finished") playbackStore.setCameraFollow(false);
  lastFrameState = frame.state;
  lastFrameTimeMs = frame.routeTimeMs;
}

function handleOverviewReady() {
  if (!map) return;
  playbackStore.setCameraFollow(false);
  const bounds = routeBounds(playbackPoints);
  if (bounds) {
    overviewCamera(map, bounds);
  }
}

function handlePlay() {
  // Start Daily playback at route coverage instead of waiting from midnight.
  // A paused cursor (including a Timeline seek) keeps its chosen time.
  if (props.dayClock && controller && (controller.currentState === "idle" || controller.currentState === "finished")) {
    controller.seek(Math.max(0, playbackPoints[0].recordedAtMs - startTimeMs.value));
  }
  playbackStore.setCameraFollow(true);
  controller?.play();
}

function handlePause() {
  controller?.pause();
}

function handleRestart() {
  mapStore.clearSelection();
  controller?.restart();
}
watch(() => playbackStore.restartRequest, () => {
  handleRestart();
  handlePlay();
});
watch(() => playbackStore.seekRequest, (request) => {
  if (!request) return;
  playbackStore.setCameraFollow(false);
  controller?.seek(request.epochMs - startTimeMs.value);
  // Re-selecting the same card still re-centers a manually panned map.
  const selected = timelineEvents.value.find(event => event.id === mapStore.selectedEventId && event.recordedAtMs === request.epochMs);
  if (map && mapReady.value && selected) {
    if (!focusTripPart(map, props.dailyView, selected.id) && selected.coordinate) {
      map.easeTo({ center: selected.coordinate, duration: 600 });
    }
  }
});

function handleSeek(timeMs: number) {
  controller?.seek(timeMs);
}

function handleSpeedChange(speed: number) {
  playbackSpeed.value = speed;
  playbackStore.setSpeed(speed as PlaybackSpeed);
  controller?.setSpeed(speed);
}

function handleToggleFollow() {
  if (!map) return;
  if (playbackStore.cameraFollow) {
    playbackStore.setCameraFollow(false);
    followCamera?.stop();
    return;
  }
  playbackStore.setCameraFollow(true);
  if (followCamera && lastFrame) {
    followCamera.enter(followTarget(playbackPoints, lastFrame));
  }
}

/**
 * A real user gesture drops out of camera-follow mode.
 * Programmatic camera moves (follow-camera easeTo, fitBounds, re-center)
 * also emit start events but carry no `originalEvent` — ignore those so
 * our own camera work doesn't cancel follow mode.
 */
function handleManualInteraction(event: { originalEvent?: unknown }) {
  if (!event.originalEvent) return;
  playbackStore.setCameraFollow(false);
  followCamera?.stop();
}

watch(
  () => playbackState.value === "playing" && playbackStore.cameraFollow,
  (following) => playbackInteraction?.setFollowing(following),
  { flush: "sync" },
);

function currentMapStyle() {
  return resolveMapStyle({
    presetId: mapPreferences.styleId,
    styleUrl: import.meta.env.VITE_MAP_STYLE_URL,
    mapTilerKey: import.meta.env.VITE_MAPTILER_KEY,
  });
}

watch(() => mapPreferences.styleId, () => {
  if (!map) return;
  controller?.pause();
  playbackStore.setCameraFollow(false);
  mapReady.value = false;
  map.setStyle(currentMapStyle(), { diff: false });
});

/** A style replacement removes custom sources and layers; restore the playback frame. */
function initializeEventDots() {
  if (!map || !props.dayClock) return;
  prepareDailyLayers(map, Boolean(controller));
  addEventDots(map, timelineEvents.value, controller ? startTimeMs.value + routeTimeMs.value : null);
  if (!eventDots) eventDots = bindEventDots(map, timelineEvents.value, mapStore.selectEvent);
}

function restoreMapStyle() {
  if (!map) return;
  addBuildings(map);
  addRoutePartLayers(map, props.dailyView);
  addRouteLayers(map, props.dailyView, playbackPoints);
  addStopLayers(map, props.dailyView);
  initializeEventDots();
  mapReady.value = true;
  if (lastFrame) applyFrame(lastFrame);
  applySelectionHighlight();
  map.easeTo({
    pitch: mapStylePreset(mapPreferences.styleId).pitch,
    duration: prefersReducedMotion() ? 0 : 600,
  });
}

/** Timeline (or playback) selection -> map highlight + flyTo. */
watch(
  () => mapStore.selectedEventId,
  (selectedId) => {
    if (!map || !mapReady.value) return;
    applySelectionHighlight();
    if (props.dayClock) eventDots?.showSelected(selectedId);
    if (focusTripPart(map, props.dailyView, selectedId)) return;
    if (focusStop(map, props.dailyView, selectedId)) return;
    const target =
      selectedId === "start"
        ? startCoordinate()
        : selectedId === "end"
          ? endCoordinate()
          : undefined;
    if (target) {
      map.easeTo({ center: target, duration: 600 });
    }
  },
);

onMounted(() => {
  if (!mapElement.value) return;
  if (props.dayClock) {
    const bounds = dayClockBounds(props.dailyView.date, props.dailyView.timezone);
    startTimeMs.value = bounds.startTimeMs;
    durationMs.value = bounds.endTimeMs - bounds.startTimeMs;
  }
  const camera = initialMapCamera({
    routeCoordinates: routeCoordinates(),
    startCoordinate: startCoordinate(),
    endCoordinate: endCoordinate(),
  });
  map = new MapLibreMap({
    container: mapElement.value,
    style: currentMapStyle(),
    ...camera,
    pitch: mapStylePreset(mapPreferences.styleId).pitch,
    maxPitch: 60,
  });
  // Heading-up compass: rotates with the map bearing so it always shows
  // the current direction of travel. Clicking it resets to north-up.
  map.addControl(
    new NavigationControl({ showCompass: true, showZoom: false, visualizePitch: true }),
    "top-left",
  );

  const input = playbackInput.value;
  if (input?.ok) {
    playbackPoints = input.points;
    const bounds = props.dayClock ? dayClockBounds(props.dailyView.date, props.dailyView.timezone) : { startTimeMs: input.points[0].recordedAtMs, endTimeMs: input.points[input.points.length - 1].recordedAtMs };
    durationMs.value = bounds.endTimeMs - bounds.startTimeMs;
    startTimeMs.value = bounds.startTimeMs;
    controller = new PlaybackController({
      points: input.points,
      startTimeMs: bounds.startTimeMs,
      endTimeMs: bounds.endTimeMs,
      speed: playbackSpeed.value,
      events: { onFrame: applyFrame, onOverviewReady: handleOverviewReady },
    });
    playbackStore.initialize({
      ...bounds,
    });
  }

  initializeWhenMapLoaded(map, () => {
    const activeMap = map;
    if (!activeMap) return;
    addBuildings(activeMap);
    addRoutePartLayers(activeMap, props.dailyView);
    addRouteLayers(activeMap, props.dailyView, playbackPoints);
    addStopLayers(activeMap, props.dailyView);
    bindStopSelection(activeMap, mapStore.selectEvent);
    bindRoutePartSelection(activeMap, mapStore.selectEvent);
    initializeEventDots();
    fitRoute(activeMap);
    if (controller) {
      playbackInteraction = new PlaybackInteraction(activeMap);
      followCamera = new FollowCamera(activeMap, {
        offsetYPx: followOffsetYPx(activeMap.getContainer().clientHeight),
      });
    }
    // Map -> selection store (the other half of the Timeline <-> Map bridge).
    // The start/end layers are only added when the daily view has those
    // events (see addRouteLayers), so only wire their click handlers then.
    if (props.dailyView.start) {
      activeMap.on("click", LAYER_START, () => mapStore.selectEvent("start"));
    }
    if (props.dailyView.end) {
      activeMap.on("click", LAYER_END, () => mapStore.selectEvent("end"));
    }
    activeMap.on("dragstart", handleManualInteraction);
    activeMap.on("zoomstart", handleManualInteraction);
    activeMap.on("rotatestart", handleManualInteraction);
    activeMap.on("style.load", restoreMapStyle);
    mapReady.value = true;
    if (props.dayClock) controller?.restart();
    if (typeof ResizeObserver !== "undefined") {
      resizeObserver = new ResizeObserver(() => activeMap.resize());
      resizeObserver.observe(activeMap.getContainer());
    }
  });
});

onBeforeUnmount(() => {
  eventDots?.dispose();
  resizeObserver?.disconnect();
  controller?.dispose();
  controller = undefined;
  followCamera?.dispose();
  followCamera = undefined;
  playbackInteraction?.dispose();
  playbackInteraction = undefined;
  playbackPoints = [];
  mapStore.clearSelection();
  playbackStore.reset();
  map?.remove();
  map = undefined;
});
</script>

<template>
  <div class="route-playback" :class="{ 'route-playback--day': dayClock }">
    <div ref="mapElement" class="route-map" aria-label="Bản đồ Daily Route" />
    <PlaybackBar
      v-if="hasPlaybackSource || dayClock"
      :state="playbackState"
      :speed="playbackSpeed"
      :route-time-ms="routeTimeMs"
      :duration-ms="durationMs"
      :start-time-ms="startTimeMs"
      :timezone="dailyView.timezone"
      :map-ready="mapReady"
      :disabled-reason="disabledReason ?? (!hasPlaybackSource ? 'Không có route để phát lại.' : null)"
      :camera-follow="playbackStore.cameraFollow"
      @play="handlePlay"
      @pause="handlePause"
      @restart="handleRestart"
      @seek="handleSeek"
      @speed-change="handleSpeedChange"
      @toggle-follow="handleToggleFollow"
    />
    <p v-if="interruption" class="playback-interruption" role="status">{{ INTERRUPTION_TEXT[interruption] }}</p>
  </div>
</template>

<style scoped src="./route-map.css"></style>

<style src="./route-map-controls.css"></style>
