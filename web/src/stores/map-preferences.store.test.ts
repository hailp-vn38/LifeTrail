import { createPinia, setActivePinia } from "pinia";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MAP_STYLE_STORAGE_KEY, useMapPreferencesStore } from "./map-preferences.store";

beforeEach(() => {
  localStorage.clear();
  setActivePinia(createPinia());
});
afterEach(() => vi.restoreAllMocks());

describe("map preferences", () => {
  it("preserves a selection across a new app session", () => {
    useMapPreferencesStore().selectStyle("streets-3d");
    setActivePinia(createPinia());
    expect(useMapPreferencesStore().styleId).toBe("streets-3d");
  });

  it("uses the configured default for unknown saved values and ignores invalid choices", () => {
    localStorage.setItem(MAP_STYLE_STORAGE_KEY, "removed-style");
    const store = useMapPreferencesStore();
    expect(store.styleId).toBe("configured");
    store.selectStyle("unknown");
    expect(store.styleId).toBe("configured");
  });

  it("still applies a choice when storage is unavailable", () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => { throw new Error("blocked"); });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => { throw new Error("blocked"); });
    const store = useMapPreferencesStore();
    store.selectStyle("streets-3d");
    expect(store.styleId).toBe("streets-3d");
    expect(store.saved).toBe(false);
  });
});
