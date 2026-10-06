<script setup lang="ts">
import { Download, MapPin, Play, RefreshCw, Smartphone } from "lucide-vue-next";
import { useRouter } from "vue-router";
import AppIconButton from "../../../components/ui/AppIconButton.vue";
import AppButton from "../../../components/ui/AppButton.vue";
import DailyMapToolbar from "./DailyMapToolbar.vue";
import { usePlaybackStore } from "../../../stores/playback.store";
import { downloadDay } from "../lib/export-day";
import type { DailyView } from "../../../api/queries/daily-view.query";
import { useDevices } from "../../../api/queries/devices.query";
import { todayForOwner } from "../../../lib/date";

const props = defineProps<{ deviceId: string; date: string; dailyView?: DailyView | null }>();
const playback = usePlaybackStore();
const emit = defineEmits<{ "date-change": [date: string]; refresh: [] }>();

const router = useRouter();
const devicesQuery = useDevices();

function onDeviceChange(event: Event) {
  const nextId = (event.target as HTMLSelectElement).value;
  const device = devicesQuery.data.value?.find((item) => item.id === nextId);
  if (!device || nextId === props.deviceId) return;
  router.push(`/devices/${nextId}/day/${todayForOwner(device.timezone)}`);
}

function onDateChange(event: Event) {
  const nextDate = (event.target as HTMLInputElement).value;
  if (nextDate && nextDate !== props.date) emit("date-change", nextDate);
}
</script>

<template>
  <header class="daily-map-header">
    <div class="daily-map-header__brand"><MapPin :size="25" /><strong>LifeTrail</strong></div>
    <DailyMapToolbar :date="date" :timezone="dailyView?.timezone" @date-change="emit('date-change', $event)" />
    <div class="daily-map-header__actions">
      <label class="daily-map-header__field">
        <Smartphone :size="18" />
        <select
          :value="deviceId"
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
      </label>
      <label class="daily-map-header__field">
        <span class="text-muted text-sm">Ngày</span>
        <input :value="date" type="date" aria-label="Chọn ngày" @change="onDateChange" />
      </label>
      <AppButton size="sm" :disabled="playback.durationMs <= 0" @click="playback.requestRestart()"><Play :size="16" /> Phát lại</AppButton>
      <details class="daily-map-header__export"><summary><Download :size="16" /> Xuất</summary><div><button :disabled="!dailyView" @click="dailyView && downloadDay(dailyView, 'gpx')">GPX</button><button :disabled="!dailyView" @click="dailyView && downloadDay(dailyView, 'csv')">CSV</button></div></details>
      <AppIconButton :icon="RefreshCw" label="Tải lại Daily Map" @click="emit('refresh')" />
    </div>
  </header>
</template>

<style scoped>
.daily-map-header { display: flex; align-items: center; flex-wrap: wrap; gap: .8rem; }
.daily-map-header__brand { display: flex; align-items: center; gap: .4rem; color: #2563eb; font-size: 1.2rem; }
.daily-map-header__actions { display: flex; align-items: center; gap: .5rem; flex-wrap: wrap; margin-left: auto; }
.daily-map-header__export { position: relative; }
.daily-map-header__export summary { display: flex; gap: .4rem; align-items: center; cursor: pointer; padding: .5rem; border: 1px solid var(--color-border); border-radius: .5rem; font-size: .8rem; }
.daily-map-header__export > div { position: absolute; right: 0; top: 100%; padding: .5rem; background: white; border: 1px solid var(--color-border); border-radius: .5rem; z-index: 5; display: flex; gap: .5rem; }
.daily-map-header__export button { padding: .4rem .6rem; }
.daily-map-header__field {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}
.daily-map-header__field select,
.daily-map-header__field input {
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  background: var(--color-surface);
  padding: 0.45rem 0.6rem;
  font-size: var(--font-size-sm);
  max-width: 12rem;
}
</style>
