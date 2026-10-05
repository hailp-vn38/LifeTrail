export type LngLat = [longitude: number, latitude: number];

export function isValidLngLat(value: unknown): value is LngLat {
  if (!Array.isArray(value) || value.length < 2) return false;
  const [longitude, latitude] = value;
  return (
    typeof longitude === "number" &&
    typeof latitude === "number" &&
    Number.isFinite(longitude) &&
    Number.isFinite(latitude) &&
    longitude >= -180 &&
    longitude <= 180 &&
    latitude >= -90 &&
    latitude <= 90
  );
}
