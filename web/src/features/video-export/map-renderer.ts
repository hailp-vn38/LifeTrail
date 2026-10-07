import { Map, LngLatBounds, type GeoJSONSource } from "maplibre-gl";
import type { DailyView } from "../../api/queries/daily-view.query";
import type { MapStyleId } from "../../map/style-presets";
import { mapStylePreset } from "../../map/style-presets";
import { resolveMapStyle } from "../../components/map-style";
import { addRoutePartLayers, PART_LINE } from "../../map/route-parts";
import { addBuildings } from "../../map/buildings";
import { addStopLayers } from "../../map/stops";
import { addRouteLayers, puckFeatures, SOURCE_CURRENT, SOURCE_PROGRESS } from "../../map/route-playback/layers";
import { progressFeatures } from "../../map/route-playback/progress";
import { FollowCamera, followOffsetYPx } from "../../map/route-playback/camera";
import { followTarget } from "../../map/route-playback/target";
import type { PlaybackFrame, PlaybackPoint } from "../../map/route-playback/types";

function waitForIdle(map: Map, signal: AbortSignal) {
  return new Promise<void>((resolve, reject) => {
    const cleanup = () => { clearTimeout(timeout); map.off("idle", ready); signal.removeEventListener("abort", abort); };
    const ready = () => { cleanup(); resolve(); };
    const abort = () => { cleanup(); reject(signal.reason); };
    const timeout = setTimeout(() => { cleanup(); reject(new Error("Bản đồ tải quá lâu. Hãy thử lại hoặc chọn kiểu bản đồ khác.")); }, 30_000);
    map.on("idle", ready);
    signal.addEventListener("abort", abort, { once: true });
    if (signal.aborted) abort();
    else if (map.loaded() && !map.isMoving()) ready();
    else map.triggerRepaint();
  });
}

export function createExportMap(container: HTMLElement, view: DailyView, points: PlaybackPoint[], styleId: MapStyleId) {
  const map = new Map({
    container, style: resolveMapStyle({ presetId: styleId, styleUrl: import.meta.env.VITE_MAP_STYLE_URL, mapTilerKey: import.meta.env.VITE_MAPTILER_KEY }),
    center: points[0].coordinate, zoom: 12, interactive: false,
    pixelRatio: 1280 / Math.max(1, container.clientWidth), canvasContextAttributes: { preserveDrawingBuffer: true },
  });
  let camera: FollowCamera | undefined;
  let frame: PlaybackFrame | undefined;
  return {
    async prepare(signal: AbortSignal) {
      await waitForIdle(map, signal);
      addBuildings(map);
      addRoutePartLayers(map, view);
      map.setPaintProperty(PART_LINE, "line-opacity", 0.3);
      addRouteLayers(map, view, points);
      addStopLayers(map, view);
      const coordinates = view.route_parts?.length ? view.route_parts.flatMap(part => part.geometry.coordinates) : points.map(point => point.coordinate);
      const bounds = new LngLatBounds(coordinates[0] as [number, number], coordinates[0] as [number, number]);
      coordinates.forEach(point => bounds.extend(point as [number, number]));
      map.fitBounds(bounds, { padding: { top: 70, bottom: 55, left: 35, right: 35 }, maxZoom: 16, duration: 0, pitch: mapStylePreset(styleId).pitch });
      camera = new FollowCamera(map, { offsetYPx: followOffsetYPx(container.clientHeight) });
      await waitForIdle(map, signal);
    },
    apply(next: PlaybackFrame, startMs: number, follow: boolean) {
      const previous = frame;
      frame = next;
      (map.getSource(SOURCE_PROGRESS) as GeoJSONSource).setData(progressFeatures(points, next, startMs + next.routeTimeMs));
      (map.getSource(SOURCE_CURRENT) as GeoJSONSource).setData(puckFeatures(next.position, next.bearing));
      if (follow && camera) {
        const target = followTarget(points, next);
        if (next.state !== "playing" || previous?.state !== "playing") camera.enter(target);
        else camera.update(target);
      }
    },
    onRender(callback: (canvas: HTMLCanvasElement, frame: PlaybackFrame, attribution: string) => void) {
      const draw = () => {
        if (!frame) return;
        // AttributionControl includes notices from loaded sources, including TileJSON metadata.
        const attribution = map.getContainer().querySelector(".maplibregl-ctrl-attrib-inner")?.textContent ?? "";
        callback(map.getCanvas(), frame, attribution.replace(/MapLibre\s*\|?\s*/g, "").trim());
      };
      map.on("render", draw);
      return () => map.off("render", draw);
    },
    onFailure(callback: (reason: Error) => void) {
      map.on("error", () => callback(new Error("Không thể tải bản đồ để xuất video. Hãy thử lại hoặc chọn kiểu bản đồ khác.")));
      map.on("webglcontextlost", () => callback(new Error("Thiết bị đã mất kết nối đồ họa. Hãy thử lại.")));
    },
    isMapTiler: () => JSON.stringify(map.getStyle().sources).includes("maptiler.com"),
    idle: (signal: AbortSignal) => waitForIdle(map, signal),
    repaint: () => map.triggerRepaint(),
    dispose: () => { camera?.dispose(); map.remove(); },
  };
}
