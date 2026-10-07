<script setup lang="ts">
import { ChevronDown, Ellipsis, Smartphone } from "lucide-vue-next";
import { useRouter } from "vue-router";
import { ref } from "vue";
import VideoExportDialog from "../../video-export/components/VideoExportDialog.vue";
import { downloadDay } from "../lib/export-day";
import type { DailyView } from "../../../api/queries/daily-view.query";
import { useDevices } from "../../../api/queries/devices.query";
import { todayForOwner } from "../../../lib/date";

const props = defineProps<{ deviceId: string; dailyView?: DailyView | null }>();
const videoView = ref<DailyView | null>(null);
const actionMenu = ref<HTMLDetailsElement>();

function exportFile(format: "gpx" | "csv") {
  if (props.dailyView) {
    void downloadDay(props.dailyView, format).catch((reason) => {
      // Canonical playback is required for export; never export display geometry.
      window.alert(reason instanceof Error ? reason.message : "Không thể xuất dữ liệu.");
    });
  }
  if (actionMenu.value) actionMenu.value.open = false;
}

function openVideo() {
  if (props.dailyView) videoView.value = JSON.parse(JSON.stringify(props.dailyView));
  if (actionMenu.value) actionMenu.value.open = false;
}

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
    <details ref="actionMenu" class="daily-map-header__export" @keydown.esc="actionMenu && (actionMenu.open = false)">
      <summary aria-label="Thao tác Daily View"><Ellipsis :size="16" /> Thao tác <ChevronDown :size="14" /></summary>
      <div>
        <button :disabled="!dailyView" @click="openVideo">Xuất video</button>
        <button :disabled="!dailyView" @click="exportFile('gpx')">Xuất GPX</button>
        <button :disabled="!dailyView" @click="exportFile('csv')">Xuất CSV</button>
      </div>
    </details>
    <VideoExportDialog v-if="videoView" :daily-view="videoView" @close="videoView = null" />
  </div>
</template>

<style scoped>
.daily-map-header__actions { display: flex; align-items: center; gap: 8px; min-width: 0; }
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
.daily-map-header__export > div { position: absolute; right: 0; top: calc(100% + 8px); z-index: 10; display: flex; flex-direction: column; min-width: 150px; gap: 4px; padding: 8px; background: var(--color-surface); border: 1px solid var(--color-border); border-radius: 10px; box-shadow: var(--shadow-popover); }
.daily-map-header__export button { padding: 8px 12px; border: 0; border-radius: 6px; background: var(--color-bg); font-size: 13px; text-align: left; cursor: pointer; }
@media (max-width: 767px) { .daily-map-header__actions { flex-wrap: wrap; } }
</style>
