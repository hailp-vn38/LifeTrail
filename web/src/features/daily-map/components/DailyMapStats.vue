<script setup lang="ts">
import type { DailyView } from "../../../api/queries/daily-view.query";
import { formatCount, formatDistance, formatDuration, formatTimestamp } from "../../../lib/format";

defineProps<{ dailyView: DailyView }>();
</script>

<template>
  <section class="daily-map-stats" aria-label="Thống kê ngày">
    <h2 class="section-title">Statistics</h2>
    <dl class="daily-map-stats__grid">
      <div class="daily-map-stats__item">
        <dt>GPS records</dt>
        <dd class="tabular">{{ formatCount(dailyView.summary.point_count) }}</dd>
      </div>
      <div class="daily-map-stats__item">
        <dt>Khoảng cách</dt>
        <dd class="tabular">{{ formatDistance(dailyView.summary.distance_m) }}</dd>
      </div>
      <div class="daily-map-stats__item">
        <dt>Thời lượng</dt>
        <dd class="tabular">{{ formatDuration(dailyView.summary.duration_s) }}</dd>
      </div>
      <div class="daily-map-stats__item">
        <dt>Bắt đầu</dt>
        <dd class="tabular">{{ formatTimestamp(dailyView.summary.first_fix_at, dailyView.timezone) }}</dd>
      </div>
      <div class="daily-map-stats__item">
        <dt>Kết thúc</dt>
        <dd class="tabular">{{ formatTimestamp(dailyView.summary.last_fix_at, dailyView.timezone) }}</dd>
      </div>
      <div class="daily-map-stats__item">
        <dt>Múi giờ</dt>
        <dd>{{ dailyView.timezone }}</dd>
      </div>
    </dl>
  </section>
</template>

<style scoped>
.daily-map-stats {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-card);
  padding: 1rem;
}
.daily-map-stats__grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.75rem;
  margin-top: 0.75rem;
}
.daily-map-stats__item {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
  padding: 0.6rem 0.75rem;
  background: var(--color-bg);
  border-radius: var(--radius-sm);
}
.daily-map-stats__item dt {
  font-size: var(--font-size-xs);
  color: var(--color-text-muted);
}
.daily-map-stats__item dd {
  font-size: var(--font-size-sm);
  font-weight: 650;
}
</style>
