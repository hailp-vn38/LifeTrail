import { defineStore } from "pinia";
import { ref } from "vue";

/**
 * Shared map selection state.
 *
 * This is the mandatory boundary between Timeline and Map (§20 of the
 * Option 2 spec): neither component calls methods on the other. Timeline
 * clicks write `selectedEventId`; the map reacts with highlight + flyTo.
 * Map feature clicks write `selectedEventId`; the timeline reacts with
 * highlight + scrollIntoView.
 */
export const useMapStore = defineStore("map", () => {
  const selectedEventId = ref<string | null>(null);

  function selectEvent(id: string | null): void {
    selectedEventId.value = id;
  }

  function clearSelection(): void {
    selectedEventId.value = null;
  }

  return { selectedEventId, selectEvent, clearSelection };
});
