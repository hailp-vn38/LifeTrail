<script setup lang="ts">
import { computed, toRef } from "vue";
import { useDevices } from "../../../api/queries/devices.query";
import { useWeekData } from "../composables/useWeekData";
import type { DailyView } from "../../../api/queries/daily-view.query";
import DailyDateContext from "./DailyDateContext.vue";
import DailyMapToolbar from "./DailyMapToolbar.vue";
import DailyMapActions from "./DailyMapActions.vue";

const props = defineProps<{ deviceId: string; date: string; dailyView?: DailyView | null }>();
const devices = useDevices();
const timezone = computed(() => props.dailyView?.timezone ?? devices.data.value?.find(device => device.id === props.deviceId)?.timezone);
const { daysWithData, refresh: refreshWeek } = useWeekData(toRef(props, "deviceId"), toRef(props, "date"), timezone);
const emit = defineEmits<{ "date-change": [date: string]; refresh: [] }>();

function refresh() {
  void refreshWeek();
  emit("refresh");
}
</script>

<template>
  <header class="daily-map-header">
    <DailyDateContext :date="date" @date-change="emit('date-change', $event)" />
    <DailyMapToolbar :days-with-data="daysWithData" :date="date" :timezone="timezone" @date-change="emit('date-change', $event)" />
    <DailyMapActions :device-id="deviceId" :daily-view="dailyView" @refresh="refresh" />
  </header>
</template>

<style scoped>
.daily-map-header {
  display: grid;
  grid-template-columns: 15rem minmax(0, 1fr) auto;
  align-items: center;
  gap: 24px;
  min-height: 68px;
  padding: 6px 20px;
  border: 1px solid var(--color-border);
  border-radius: 16px;
  background: var(--color-surface);
  box-shadow: 0 2px 12px rgb(23 32 51 / 3%);
  flex-shrink: 0;
}
@media (max-width: 1399px) {
  .daily-map-header { grid-template-columns: 15rem minmax(0, 1fr); gap: 16px; }
  .daily-map-header > :last-child { grid-column: 1 / -1; }
}
@media (max-width: 767px) {
  .daily-map-header { grid-template-columns: minmax(0, 1fr); padding: 8px 16px; }
}
</style>
