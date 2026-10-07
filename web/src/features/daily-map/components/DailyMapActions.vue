<script setup lang="ts">
import { ChevronDown, Download, Play, RefreshCw, Smartphone } from "lucide-vue-next";
import { useRouter } from "vue-router";
import AppIconButton from "../../../components/ui/AppIconButton.vue";
import AppButton from "../../../components/ui/AppButton.vue";
import { usePlaybackStore } from "../../../stores/playback.store";
import { downloadDay } from "../lib/export-day";
import type { DailyView } from "../../../api/queries/daily-view.query";
import { useDevices } from "../../../api/queries/devices.query";
import { todayForOwner } from "../../../lib/date";

const props = defineProps<{ deviceId: string; dailyView?: DailyView | null }>();
const playback = usePlaybackStore();
const emit = defineEmits<{ refresh: [] }>();

const router = useRouter();
const devicesQuery = useDevices();

function onDeviceChange(event: Event) {
  const nextId = (event.target as HTMLSelectElement).value;
  const device = devicesQuery.data.value?.find((item) => item.id === nextId);
  if (!device || nextId === props.deviceId) return;
  router.push(`/devices/${nextId}/day/${todayForOwner(device.timezone)}`);
}
</script>

<template>
  <div class="daily-map-header__actions">
    <label class="daily-map-header__field">
      <Smartphone :size="16" aria-hidden="true" />
      <select
        :value="deviceId"
        :title="devicesQuery.data.value?.find(device => device.id === deviceId)?.name ?? deviceId"
        :disabled="devicesQuery.isPending.value"
        aria-label="Chọn device"
        @change="onDeviceChange"
      >
        <option v-if="!devicesQuery.data.value?.some(device => device.id === deviceId)" :value="deviceId">{{ deviceId }}</option>
        <option
          v-for="device in devicesQuery.data.value ?? []"
          :key="device.id"
          :value="device.id"
        >
          {{ device.name }}
        </option>
      </select>
      <ChevronDown :size="14" class="daily-map-header__select-chevron" aria-hidden="true" />
    </label>
    <AppButton variant="primary" size="sm" :disabled="!dailyView || playback.durationMs <= 0" @click="playback.requestRestart()"><Play :size="16" /> Phát lại</AppButton>
    <details class="daily-map-header__export"><summary><Download :size="16" /> Xuất</summary><div><button :disabled="!dailyView" @click="dailyView && downloadDay(dailyView, 'gpx')">GPX</button><button :disabled="!dailyView" @click="dailyView && downloadDay(dailyView, 'csv')">CSV</button></div></details>
    <AppIconButton :icon="RefreshCw" label="Tải lại Daily Map" @click="emit('refresh')" />
  </div>
</template>

<style scoped>
.daily-map-header__actions { display: flex; align-items: center; gap: 8px; min-width: 0; }
.daily-map-header__actions :deep(.app-button) { height: 38px; padding: 0 12px; font-size: 13px; white-space: nowrap; }
.daily-map-header__actions :deep(.app-icon-button) { width: 38px; height: 38px; border-radius: 10px; }
.daily-map-header__field { position: relative; display: flex; align-items: center; gap: 8px; color: var(--color-text-muted); }
.daily-map-header__field > svg { flex-shrink: 0; }
.daily-map-header__field select {
  appearance: none;
  width: 196px;
  height: 38px;
  padding: 0 28px 0 10px;
  border: 1px solid var(--color-border);
  border-radius: 10px;
  background: #f8fafc;
  color: var(--color-text);
  font-size: 13px;
  text-overflow: ellipsis;
  cursor: pointer;
}
.daily-map-header__select-chevron { position: absolute; right: 10px; pointer-events: none; }
.daily-map-header__export { position: relative; }
.daily-map-header__export summary { display: flex; align-items: center; gap: 8px; height: 38px; padding: 0 12px; border: 1px solid var(--color-border); border-radius: 10px; font-size: 13px; cursor: pointer; list-style: none; }
.daily-map-header__export summary::-webkit-details-marker { display: none; }
.daily-map-header__export summary:hover { background: var(--color-bg); }
.daily-map-header__export > div { position: absolute; right: 0; top: calc(100% + 8px); z-index: 10; display: flex; gap: 8px; padding: 8px; background: var(--color-surface); border: 1px solid var(--color-border); border-radius: 10px; box-shadow: var(--shadow-popover); }
.daily-map-header__export button { padding: 8px 12px; border: 0; border-radius: 6px; background: var(--color-bg); font-size: 12px; }
@media (max-width: 767px) { .daily-map-header__actions { flex-wrap: wrap; } }
</style>
