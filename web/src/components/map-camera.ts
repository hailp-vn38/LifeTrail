export type MapCoordinate = [number, number];

interface InitialMapCameraInput {
  routeCoordinates: MapCoordinate[];
  startCoordinate?: MapCoordinate;
  endCoordinate?: MapCoordinate;
}

export const INITIAL_MAP_ZOOM = 14;

export const routeFitOptions = {
  padding: 48,
  maxZoom: 15,
  duration: 350,
} as const;

export function initialMapCamera({
  routeCoordinates,
  startCoordinate,
  endCoordinate,
}: InitialMapCameraInput): { center: MapCoordinate; zoom: number } | undefined {
  const center = startCoordinate ?? routeCoordinates[0] ?? endCoordinate;
  return center ? { center, zoom: INITIAL_MAP_ZOOM } : undefined;
}
