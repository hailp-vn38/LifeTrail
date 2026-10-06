<script setup lang="ts">
import { MapPin, Route, Clock } from 'lucide-vue-next';
import type { DailyView } from '../../../api/queries/daily-view.query';
import { formatDistance, formatDuration } from '../../../lib/format';
defineProps<{ dailyView: DailyView }>();
</script>
<template>
  <div class="timeline-summary" aria-label="Tóm tắt ngày">
    <div><MapPin :size="14" /><strong>{{ dailyView.summary.stop_count ?? 0 }}</strong><span>điểm dừng</span></div>
    <div><Route :size="14" /><strong>{{ formatDistance(dailyView.summary.distance_m) }}</strong><span>quãng đường</span></div>
    <div><Clock :size="14" /><strong>{{ formatDuration(dailyView.processing_state === 'processed' ? (dailyView.summary.trip_duration_s ?? 0) : dailyView.summary.duration_s) }}</strong><span>{{ dailyView.processing_state === 'processed' ? 'trong Trip¹' : 'thời lượng' }}</span></div>
    <p v-if="dailyView.processing_state === 'processed'">¹ Bao gồm dừng ngắn trong Trip.</p>
  </div>
</template>
<style scoped>
.timeline-summary {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: .25rem;
  padding: .5rem .35rem;
  background: #eff6ff;
  border: 1px solid #dbeafe;
  border-radius: .5rem;
  flex: none;
}
.timeline-summary div {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: .15rem;
  text-align: center;
}
.timeline-summary svg { color: #2563eb; }
.timeline-summary strong { font-size: .75rem; font-variant-numeric: tabular-nums; }
.timeline-summary span, .timeline-summary p { font-size: .625rem; color: var(--color-text-muted); }
.timeline-summary p { grid-column: 1 / -1; margin: .15rem 0 0; text-align: center; }
</style>
