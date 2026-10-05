<script setup lang="ts">
import type { DailyView } from "../api/daily-views";
import { formatDistance, formatDuration, formatTimestamp } from "../presentation/daily-summary";

defineProps<{ dailyView: DailyView }>();
</script>

<template>
  <dl class="summary-grid">
    <div><dt>GPS Records</dt><dd>{{ dailyView.summary.point_count }}</dd></div>
    <div><dt>Khoảng cách</dt><dd>{{ formatDistance(dailyView.summary.distance_m) }}</dd></div>
    <div><dt>Thời lượng</dt><dd>{{ formatDuration(dailyView.summary.duration_s) }}</dd></div>
    <div><dt>Điểm đầu</dt><dd>{{ formatTimestamp(dailyView.summary.first_fix_at, dailyView.timezone) }}</dd></div>
    <div><dt>Điểm cuối</dt><dd>{{ formatTimestamp(dailyView.summary.last_fix_at, dailyView.timezone) }}</dd></div>
  </dl>
</template>
