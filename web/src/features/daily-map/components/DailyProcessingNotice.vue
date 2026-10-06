<script setup lang="ts">
import { computed } from "vue";
import type { DailyView } from "../../../api/queries/daily-view.query";

const props = defineProps<{ dailyView: DailyView }>();
const label = computed(() => {
  switch (props.dailyView.processing?.state) {
    case "queued": return "Đang chờ xử lý";
    case "running": return "Đang xử lý";
    case "failed": return "Xử lý chưa hoàn tất";
    default: return props.dailyView.processing_state === "processed" ? "Đã xử lý" : "Raw GPS";
  }
});
</script>

<template>
  <section class="processing-notice" role="status" aria-live="polite">
    <strong>{{ label }}</strong>
    <span v-if="dailyView.processing_state === 'raw' && label !== 'Raw GPS'"> · Raw GPS</span>
    <p v-if="dailyView.processing?.data_freshness === 'stale'">Đang hiển thị kết quả đã xử lý gần nhất; dữ liệu mới chưa được cập nhật.</p>
    <p v-if="dailyView.evidence_state === 'insufficient'">Chưa đủ dữ liệu để xác định hoạt động</p>
    <p v-if="dailyView.processing?.deferred_reason">Dữ liệu GPS vẫn có thể xem; phân tích hoạt động cho ngày này chưa khả dụng.</p>
  </section>
</template>

<style scoped>
.processing-notice { padding: 0.75rem 1rem; background: var(--color-surface); border: 1px solid var(--color-border); border-radius: var(--radius-md); }
.processing-notice p { margin-top: 0.35rem; color: var(--color-text-muted); }
</style>
