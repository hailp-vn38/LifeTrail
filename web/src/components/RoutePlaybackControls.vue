<script setup lang="ts">
import { computed } from "vue";
import { PLAYBACK_SPEEDS } from "../map/route-playback/controller";
import type { PlaybackState } from "../map/route-playback/types";

const props = defineProps<{
  state: PlaybackState;
  speed: number;
  routeTimeMs: number;
  durationMs: number;
  /** GPS epoch ms of the first route point, for the clock display. */
  startTimeMs: number;
  timezone: string;
  mapReady: boolean;
  /** Controlled reason when playback must stay disabled; null when usable. */
  disabledReason: string | null;
}>();

const emit = defineEmits<{
  play: [];
  pause: [];
  restart: [];
  seek: [routeTimeMs: number];
  "speed-change": [speed: number];
}>();

const canInteract = computed(() => props.mapReady && !props.disabledReason);

const progressPermille = computed(() => {
  if (!(props.durationMs > 0)) return 0;
  return Math.round((props.routeTimeMs / props.durationMs) * 1000);
});

const playLabel = computed(() => {
  if (props.state === "playing") return "Tạm dừng";
  if (props.state === "finished") return "Phát lại";
  return "Phát";
});

const currentClockTime = computed(() =>
  formatClockTime(props.startTimeMs + Math.floor(props.routeTimeMs / 1000) * 1000, props.timezone),
);
const totalDuration = computed(() => formatDuration(props.durationMs));

function formatClockTime(epochMs: number, timezone: string): string {
  try {
    return new Intl.DateTimeFormat("vi-VN", {
      timeZone: timezone,
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
      hour12: false,
    }).format(new Date(epochMs));
  } catch {
    return new Date(epochMs).toISOString().slice(11, 19);
  }
}

function formatDuration(totalMs: number): string {
  const totalSeconds = Math.max(0, Math.round(totalMs / 1000));
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = String(Math.floor((totalSeconds % 3600) / 60)).padStart(2, "0");
  const seconds = String(totalSeconds % 60).padStart(2, "0");
  return hours > 0 ? `${hours}:${minutes}:${seconds}` : `${minutes}:${seconds}`;
}

function onPlayPause() {
  if (props.state === "playing") emit("pause");
  else emit("play");
}

function onSeekInput(event: Event) {
  const permille = Number((event.target as HTMLInputElement).value);
  if (!Number.isFinite(permille)) return;
  emit("seek", (permille / 1000) * props.durationMs);
}

function onSpeedChange(event: Event) {
  const speed = Number((event.target as HTMLSelectElement).value);
  if (Number.isFinite(speed) && speed > 0) emit("speed-change", speed);
}
</script>

<template>
  <div class="playback-controls" aria-label="Điều khiển phát lại route">
    <div class="playback-buttons">
      <button type="button" :disabled="!canInteract" :aria-label="playLabel" @click="onPlayPause">
        {{ playLabel }}
      </button>
      <button type="button" :disabled="!canInteract" aria-label="Chạy lại từ đầu" @click="emit('restart')">
        Chạy lại
      </button>
      <label class="speed-label">
        Tốc độ
        <select :value="speed" :disabled="!canInteract" aria-label="Tốc độ phát lại" @change="onSpeedChange">
          <option v-for="option in PLAYBACK_SPEEDS" :key="option" :value="option">
            {{ option }}x
          </option>
        </select>
      </label>
    </div>
    <div class="playback-timeline">
      <input
        type="range"
        min="0"
        max="1000"
        step="1"
        :value="progressPermille"
        :disabled="!canInteract"
        aria-label="Dòng thời gian phát lại"
        @input="onSeekInput"
      />
      <p class="playback-time">
        <span>{{ currentClockTime }}</span>
        <span aria-hidden="true"> / </span>
        <span>{{ totalDuration }}</span>
      </p>
    </div>
    <p v-if="disabledReason" class="playback-note" role="note">{{ disabledReason }}</p>
  </div>
</template>

<style scoped>
.playback-controls {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  margin-top: 0.75rem;
  padding: 0.75rem;
  border: 1px solid #e5e7eb;
  border-radius: 0.5rem;
  background: #f9fafb;
}
.playback-buttons {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-wrap: wrap;
}
.playback-buttons button {
  padding: 0.375rem 0.875rem;
  border: 1px solid #d1d5db;
  border-radius: 0.375rem;
  background: #ffffff;
  cursor: pointer;
}
.playback-buttons button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.speed-label {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  margin-left: auto;
}
.playback-timeline {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}
.playback-timeline input[type="range"] {
  flex: 1;
}
.playback-time {
  margin: 0;
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}
.playback-note {
  margin: 0;
  color: #6b7280;
  font-size: 0.875rem;
}
</style>
