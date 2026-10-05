import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { DEFAULT_MAP_STYLE_ID, isMapStyleId, type MapStyleId } from "../map/style-presets";

export const MAP_STYLE_STORAGE_KEY = "lifetrail.map-style";

function loadStyle(): MapStyleId {
  try {
    const value = localStorage.getItem(MAP_STYLE_STORAGE_KEY);
    return isMapStyleId(value) ? value : DEFAULT_MAP_STYLE_ID;
  } catch {
    return DEFAULT_MAP_STYLE_ID;
  }
}

/** Browser display preferences survive map unmounts and page reloads. */
export const useMapPreferencesStore = defineStore("map-preferences", () => {
  const selectedStyle = ref(loadStyle());
  const styleId = computed(() => selectedStyle.value);
  const saved = ref(true);

  function selectStyle(value: string): void {
    if (!isMapStyleId(value)) return;
    selectedStyle.value = value;
    try {
      localStorage.setItem(MAP_STYLE_STORAGE_KEY, value);
      saved.value = true;
    } catch {
      saved.value = false;
    }
  }

  return { styleId, saved, selectStyle };
});
