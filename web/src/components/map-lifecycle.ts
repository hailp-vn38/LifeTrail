interface LoadableMap {
  loaded(): boolean;
  once(event: "load", listener: () => void): unknown;
}

export function initializeWhenMapLoaded(
  map: LoadableMap,
  initialize: () => void,
): void {
  if (map.loaded()) {
    initialize();
    return;
  }

  map.once("load", initialize);
}
