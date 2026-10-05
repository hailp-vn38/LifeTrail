<script setup lang="ts">
import { LocateFixed, Pause, Play, RotateCcw } from "lucide-vue-next";
import { computed } from "vue";
import AppIconButton from "../../../components/ui/AppIconButton.vue";
import { PLAYBACK_SPEEDS } from "../../../map/route-playback/controller";
import type { PlaybackState } from "../../../map/route-playback/types";

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
  /** Course-up follow camera is currently tracking the position. */
  cameraFollow: boolean;
}>();

const emit = defineEmits<{
  play: [];
  pause: [];
  restart: [];
  seek: [routeTimeMs: number];
  "speed-change": [speed: number];
  "toggle-follow": [];
}>();

const canInteract = computed(() => props.mapReady && !props.disabledReason);
const canFollow = computed(() => props.mapReady && !props.disabledReason);

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
  <div class="playback-bar" aria-label="Điều khiển phát lại route">
    <div class="playback-bar__row">
      <div class="playback-bar__buttons">
        <AppIconButton
          :icon="state === 'playing' ? Pause : Play"
          :label="playLabel"
          variant="primary"
          :disabled="!canInteract"
          @click="onPlayPause"
        />
        <AppIconButton
          :icon="RotateCcw"
          label="Chạy lại từ đầu"
          :disabled="!canInteract"
          @click="emit('restart')"
        />
        <AppIconButton
          v-if="canFollow"
          :icon="LocateFixed"
          :label="cameraFollow ? 'Tắt theo dõi vị trí' : 'Theo dõi vị trí'"
          :active="cameraFollow"
          :disabled="!canInteract"
          @click="emit('toggle-follow')"
        />
      </div>
      <div class="playback-bar__scrubber">
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
      </div>
      <p class="playback-bar__time tabular">
        <span>{{ currentClockTime }}</span>
        <span aria-hidden="true"> / </span>
        <span>{{ totalDuration }}</span>
      </p>
      <label class="playback-bar__speed">
        <span class="text-muted text-sm">Tốc độ</span>
        <select
          :value="speed"
          :disabled="!canInteract"
          aria-label="Tốc độ phát lại"
          @change="onSpeedChange"
        >
          <option v-for="option in PLAYBACK_SPEEDS" :key="option" :value="option">
            {{ option }}x
          </option>
        </select>
      </label>
    </div>
    <p v-if="disabledReason" class="playback-note" role="note">{{ disabledReason }}</p>
  </div>
</template>

<style scoped>
.playback-bar {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-surface);
  box-shadow: var(--shadow-card);
}
.playback-bar__row {
  display: flex;
  align-items: center;
  gap: 0.9rem;
  flex-wrap: wrap;
}
.playback-bar__buttons {
  display: flex;
  align-items: center;
  gap: 0.4rem;
}
.playback-bar__scrubber {
  flex: 1;
  display: flex;
  min-width: 8rem;
}
.playback-bar__scrubber input[type="range"] {
  width: 100%;
  accent-color: var(--color-primary);
}
.playback-bar__time {
  margin: 0;
  font-size: var(--font-size-sm);
  white-space: nowrap;
}
.playback-bar__speed {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  margin-left: auto;
}
.playback-bar__speed select {
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  background: var(--color-surface);
  padding: 0.3rem 0.5rem;
  font-size: var(--font-size-sm);
}
.playback-note {
  margin: 0;
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}
</style>
