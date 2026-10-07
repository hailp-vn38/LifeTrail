<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import type { DailyView } from "../../../api/queries/daily-view.query";
import { getPlayback } from "../../../api/queries/playback.query";
import type { RoutePart } from "../../activity/model";
import { PLAYBACK_SPEEDS } from "../../../map/route-playback/controller";
import { useMapPreferencesStore } from "../../../stores/map-preferences.store";
import { MAX_VIDEO_SECONDS, videoDuration, videoInput } from "../export-plan";
import { downloadVideo, recordingMime } from "../media-recorder";
import { recordVideo } from "../record-video";

const props = defineProps<{ dailyView: DailyView }>();
const emit = defineEmits<{ close: [] }>();
const dialog = ref<HTMLDialogElement>();
const mapContainer = ref<HTMLDivElement>();
const preferences = useMapPreferencesStore();
const styleId = preferences.styleId;
const speed = ref(100);
const follow = ref(false);
const progress = ref(0);
const state = ref("idle");
const error = ref("");
const supportError = ref("");
const format = ref("");
const videoUrl = ref("");
const parts = ref<RoutePart[] | undefined>();
let abort: AbortController | undefined;
let firstMs = 0;
let totalSeconds = 0;
const from = ref(0);
const until = ref(totalSeconds);

/** Load canonical playback geometry lazily, then size the export window. */
async function initialize() {
  try {
    if (props.dailyView.route_parts?.length) {
      const playback = await getPlayback(
        props.dailyView.device_id,
        props.dailyView.date,
        props.dailyView.provenance?.manifest_version,
      );
      parts.value = playback.route_parts;
    }
    const points = videoInput(props.dailyView, parts.value);
    firstMs = points[0].recordedAtMs;
    totalSeconds = (points.at(-1)!.recordedAtMs - firstMs) / 1000;
    until.value = totalSeconds;
    const mime = recordingMime();
    format.value = mime.startsWith("video/mp4") ? "MP4" : "WebM";
  } catch (reason) {
    supportError.value = reason instanceof Error ? reason.message : String(reason);
  }
}
const busy = computed(() => ["preparing", "recording", "finalizing"].includes(state.value));
const duration = computed(() => {
  try { return videoDuration(firstMs + from.value * 1000, firstMs + until.value * 1000, speed.value); }
  catch { return 0; }
});
const invalid = computed(() => supportError.value || (duration.value <= 0 ? "Chọn thời gian kết thúc sau thời gian bắt đầu." : duration.value > MAX_VIDEO_SECONDS ? "Video tối đa 10 phút. Hãy tăng tốc độ hoặc rút ngắn khoảng xuất." : ""));
const clock = (offset: number) => new Intl.DateTimeFormat("vi-VN", { timeZone: props.dailyView.timezone, hour: "2-digit", minute: "2-digit", second: "2-digit", hour12: false }).format(firstMs + offset * 1000);
const durationLabel = computed(() => {
  const seconds = Math.ceil(duration.value);
  return `${Math.floor(seconds / 60)} phút ${seconds % 60} giây`;
});
const stateLabels: Record<string, string> = { preparing: "Đang tải bản đồ…", recording: "Đang ghi video…", finalizing: "Đang hoàn tất video…", done: "Đã tải video.", cancelled: "Đã hủy xuất video." };
const stateLabel = computed(() => stateLabels[state.value] ?? "");

async function start() {
  if (invalid.value || busy.value || !mapContainer.value) return;
  error.value = "";
  progress.value = 0;
  state.value = "preparing";
  if (videoUrl.value) URL.revokeObjectURL(videoUrl.value);
  videoUrl.value = "";
  abort = new AbortController();
  try {
    await nextTick();
    abort.signal.throwIfAborted();
    const blob = await recordVideo({
      view: props.dailyView, parts: parts.value, container: mapContainer.value, styleId,
      startMs: firstMs + from.value * 1000, endMs: firstMs + until.value * 1000,
      speed: speed.value, follow: follow.value, signal: abort.signal,
      onState: value => { state.value = value; }, onProgress: value => { progress.value = value; },
    });
    downloadVideo(blob, `lifetrail-${props.dailyView.device_id}-${props.dailyView.date}`);
    videoUrl.value = URL.createObjectURL(blob);
    state.value = "done";
  } catch (reason) {
    if (abort.signal.aborted) state.value = "cancelled";
    else { state.value = "error"; error.value = reason instanceof Error ? reason.message : String(reason); }
  }
}
function close() { abort?.abort(); emit("close"); }
onMounted(() => { dialog.value?.showModal(); void initialize(); });
onBeforeUnmount(() => { abort?.abort(); if (videoUrl.value) URL.revokeObjectURL(videoUrl.value); });
</script>

<template>
  <dialog ref="dialog" class="video-export" aria-labelledby="video-export-title" @cancel.prevent="close">
    <header><h2 id="video-export-title">Xuất video · {{ dailyView.date }}</h2><button aria-label="Đóng" @click="close">×</button></header>
    <p>720p · 30 FPS · {{ format || 'Video' }} · Không âm thanh. Giữ tab mở trong khi xuất.</p>
    <fieldset :disabled="busy || Boolean(supportError)">
      <label>Từ {{ clock(from) }}<input v-model.number="from" type="range" min="0" :max="totalSeconds" step="1" aria-label="Thời gian bắt đầu video" /></label>
      <label>Đến {{ clock(until) }}<input v-model.number="until" type="range" min="0" :max="totalSeconds" step="1" aria-label="Thời gian kết thúc video" /></label>
      <div class="video-export__options">
        <label>Tốc độ <select v-model.number="speed"><option v-for="value in PLAYBACK_SPEEDS" :key="value" :value="value">{{ value }}x</option></select></label>
        <label>Camera <select v-model="follow"><option :value="false">Tổng quan</option><option :value="true">Theo hành trình</option></select></label>
      </div>
    </fieldset>
    <p v-if="duration > 0">Thời lượng dự kiến: {{ durationLabel }}. Thời gian ghi tương đương độ dài video.</p>
    <div ref="mapContainer" class="video-export__preview" aria-label="Bản đồ xuất video">
      <span v-if="!busy && !videoUrl">Bản đồ sẽ hiển thị khi bắt đầu xuất.</span>
      <video v-if="videoUrl" :src="videoUrl" controls playsinline aria-label="Video đã xuất" />
    </div>
    <p v-if="invalid || error" role="alert">{{ invalid || error }}</p>
    <div v-if="busy"><progress :value="progress" max="1" aria-label="Tiến độ xuất video" /> {{ Math.round(progress * 100) }}%</div>
    <p role="status">{{ stateLabel }}</p>
    <footer><button @click="busy ? abort?.abort() : close()">{{ busy ? 'Hủy xuất' : 'Đóng' }}</button><button :disabled="busy || Boolean(invalid)" @click="start">{{ busy ? 'Đang xuất…' : 'Xuất video' }}</button></footer>
  </dialog>
</template>

<style scoped>
.video-export { width: min(680px, calc(100vw - 32px)); max-height: 90vh; overflow: auto; padding: 24px; border: 1px solid var(--color-border); border-radius: 16px; background: var(--color-surface); color: var(--color-text); }
.video-export::backdrop { background: rgb(15 23 42 / 55%); }
header, footer, .video-export__options { display: flex; align-items: center; justify-content: space-between; gap: 16px; }
h2 { margin: 0; font-size: 20px; }
p { font-size: 13px; line-height: 1.6; }
fieldset { border: 0; padding: 0; display: grid; gap: 12px; }
label { display: grid; gap: 6px; font-size: 13px; }
input { width: 100%; }
select, button { border: 1px solid var(--color-border); padding: 8px 12px; border-radius: 8px; background: var(--color-surface); color: var(--color-text); }
button { cursor: pointer; }
button:disabled { opacity: .5; cursor: default; }
footer { justify-content: flex-end; }
footer button:last-child { background: #2563eb; color: white; }
.video-export__preview { aspect-ratio: 16 / 9; margin-top: 16px; background: #f1f5f9; position: relative; overflow: hidden; border-radius: 8px; }
.video-export__preview > span { display: block; padding: 24px; color: #64748b; font-size: 13px; }
.video-export__preview > video { width: 100%; height: 100%; display: block; }
progress { width: 80%; }
[role="alert"] { color: #b91c1c; }
@media (max-width: 600px) { .video-export { padding: 16px; } }
</style>
