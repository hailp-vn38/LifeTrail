<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import AppErrorState from "../../../components/ui/AppErrorState.vue";
import { useUiStore } from "../../../stores/ui.store";
import { todayForOwner } from "../../../lib/date";
import TimelinePanel from "../../timeline/components/TimelinePanel.vue";
import TimelineStats from "../../timeline/components/TimelineStats.vue";
import DailyMapCanvas from "../components/DailyMapCanvas.vue";
import DailyMapEmpty from "../components/DailyMapEmpty.vue";
import DailyMapHeader from "../components/DailyMapHeader.vue";
import DailyMapLoading from "../components/DailyMapLoading.vue";
import DailyMapToolbar from "../components/DailyMapToolbar.vue";
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

const { query, isNotFound, isEmpty } = useDailyMap(deviceId, date);
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
  <div class="page page--wide">
    <DailyMapHeader
      :device-id="deviceId"
      :date="date"
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

    <div v-else-if="query.data.value" class="daily-map-workspace">
      <div class="map-column">
        <DailyMapToolbar
          :date="date"
          :timezone="timezone"
          @date-change="(next) => navigate(deviceId, next)"
        />
        <DailyMapEmpty
          v-if="isEmpty"
          @go-today="navigate(deviceId, todayForOwner(timezone ?? 'UTC'))"
        />
        <DailyMapCanvas v-else :daily-view="query.data.value" />
      </div>
      <div class="timeline-column">
        <TimelinePanel :daily-view="query.data.value" />
        <TimelineStats :daily-view="query.data.value" />
      </div>
    </div>
  </div>
</template>
