import { VERSATILES_STYLE_PRESETS } from "./versatiles-presets";

/** Basemap choices shared by Settings and the map renderer. */
export const MAP_STYLE_PRESETS = [
  { id: "configured", label: "Mặc định", description: "Dùng bản đồ mặc định của LifeTrail.", url: undefined, pitch: 0, provider: "LifeTrail / MapTiler" },
  { id: "streets-3d", label: "Đường phố 3D", description: "Góc nhìn nghiêng với các tòa nhà 3D, phù hợp để phát lại tuyến đường.", url: "https://api.maptiler.com/maps/streets-v4/style.json", pitch: 55, provider: "LifeTrail / MapTiler" },
  { id: "streets", label: "Đường phố", description: "Bản đồ đường phố nhìn từ trên xuống, dễ đọc tên đường.", url: "https://api.maptiler.com/maps/streets-v4/style.json", pitch: 0, provider: "LifeTrail / MapTiler" },
  { id: "hybrid", label: "Vệ tinh", description: "Ảnh vệ tinh kết hợp tên đường và địa điểm.", url: "https://api.maptiler.com/maps/hybrid/style.json", pitch: 0, provider: "LifeTrail / MapTiler" },
  ...VERSATILES_STYLE_PRESETS,
] as const;

export type MapStyleId = (typeof MAP_STYLE_PRESETS)[number]["id"];
export const DEFAULT_MAP_STYLE_ID: MapStyleId = "configured";

export function isMapStyleId(value: unknown): value is MapStyleId {
  return MAP_STYLE_PRESETS.some((preset) => preset.id === value);
}

export function mapStylePreset(id: MapStyleId) {
  return MAP_STYLE_PRESETS.find((preset) => preset.id === id) ?? MAP_STYLE_PRESETS[0];
}
