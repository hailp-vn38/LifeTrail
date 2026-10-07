<script setup lang="ts">
import "../styles/daily-map-workspace.css";
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import AppButton from "../../../components/ui/AppButton.vue";
import AppErrorState from "../../../components/ui/AppErrorState.vue";
import { useUiStore } from "../../../stores/ui.store";
import { todayForOwner } from "../../../lib/date";
import TimelinePanel from "../../timeline/components/TimelinePanel.vue";
import DailyMapCanvas from "../components/DailyMapCanvas.vue";
import DailyMapEmpty from "../components/DailyMapEmpty.vue";
import DailyMapHeader from "../components/DailyMapHeader.vue";
import DailyMapLoading from "../components/DailyMapLoading.vue";
import { useDailyMap } from "../composables/useDailyMap";

const route = useRoute();
const router = useRouter();
const ui = useUiStore();

const deviceId = ref(String(route.params.deviceId));
const date = ref(String(route.params.date));

watch(
  () => [route.params.deviceId, route.params.date],
  ([nextDeviceId, nextDate]) => {
    deviceId.value = String(nextDeviceId);
    date.value = String(nextDate);
  },
);

const { query, isNotFound, isEmpty, rawMode } = useDailyMap(deviceId, date);
const timezone = computed(() => query.data.value?.timezone);

function navigate(device: string, day: string) {
  router.push(`/devices/${device}/day/${day}`);
}

// Remember this route so the "Daily Map" sidebar entry can deep-link back.
onMounted(() => ui.setLastDailyMapRoute(route.fullPath));
watch(
  () => route.fullPath,
  (fullPath) => ui.setLastDailyMapRoute(fullPath),
);
</script>

<template>
  <div class="page page--wide daily-map-page">
    <DailyMapHeader
      :device-id="deviceId"
      :date="date"
      :daily-view="query.data.value"
      @date-change="(next) => navigate(deviceId, next)"
      @refresh="query.refetch()"
    />

    <DailyMapLoading v-if="query.isPending.value" />

    <AppErrorState
      v-else-if="isNotFound"
      title="Không tìm thấy Device"
      message="Device này không tồn tại hoặc đã bị xóa."
      retry-label="Về danh sách Devices"
      @retry="router.push('/devices')"
    />

    <AppErrorState
      v-else-if="query.isError.value"
      title="Không thể tải Daily View"
      message="Đã có lỗi khi tải dữ liệu. Kiểm tra kết nối rồi thử lại."
      @retry="query.refetch()"
    />

    <template v-else-if="query.data.value">
      <div class="daily-map-workspace">
        <div class="map-column">
          <AppButton
            class="daily-map-page__raw-toggle"
            :aria-label="rawMode ? 'Xem hoạt động' : 'Xem Raw GPS'"
            :aria-pressed="rawMode"
            variant="ghost"
            size="sm"
            @click="rawMode = !rawMode"
          >{{ rawMode ? 'Hoạt động' : 'Raw GPS' }}</AppButton>

          <DailyMapEmpty
            v-if="isEmpty"
            @go-today="navigate(deviceId, todayForOwner(timezone ?? 'UTC'))"
          />
          <DailyMapCanvas v-else-if="query.data.value.evidence_state !== 'insufficient'" :daily-view="query.data.value" />
        </div>
        <div class="timeline-column">
          <TimelinePanel :daily-view="query.data.value" />
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.daily-map-page { height: calc(100dvh - 6rem); min-height: 34rem; gap: 8px; }
.map-column { position: relative; }
.daily-map-page__raw-toggle { position: absolute; top: 8px; left: 64px; z-index: 2; background: var(--color-surface); border: 1px solid var(--color-border); }
@media (max-width: 767px) {
  .daily-map-page { height: auto; min-height: 0; }
  .timeline-column :deep(.timeline-panel) { height: auto; overflow: visible; }
}
</style>
