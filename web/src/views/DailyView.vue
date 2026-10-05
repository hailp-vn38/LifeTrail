<script setup lang="ts">
import { computed } from "vue";
import { RouterLink, useRoute, useRouter } from "vue-router";
import { ApiRequestError } from "../api/request-error";
import DailySummary from "../components/DailySummary.vue";
import RouteMap from "../components/RouteMap.vue";
import { useDailyView } from "../queries/use-daily-view";

const route = useRoute();
const router = useRouter();
const deviceId = computed(() => String(route.params.deviceId));
const date = computed(() => String(route.params.date));
const dailyViewQuery = useDailyView(deviceId, date);
const isNotFound = computed(
  () => dailyViewQuery.error.value instanceof ApiRequestError && dailyViewQuery.error.value.status === 404,
);

function changeDate(event: Event) {
  const nextDate = (event.target as HTMLInputElement).value;
  if (nextDate) router.push(`/devices/${deviceId.value}/day/${nextDate}`);
}
</script>

<template>
  <section>
    <RouterLink to="/">← Devices</RouterLink>
    <header class="daily-header">
      <div>
        <h1>Daily View</h1>
        <p>{{ date }}</p>
      </div>
      <label>
        Ngày
        <input :value="date" type="date" @change="changeDate" />
      </label>
    </header>

    <p v-if="dailyViewQuery.isPending.value" role="status">Đang tải Daily View…</p>
    <p v-else-if="isNotFound" role="alert">Không tìm thấy Device này.</p>
    <p v-else-if="dailyViewQuery.isError.value" role="alert">Không thể tải Daily View.</p>
    <template v-else-if="dailyViewQuery.data.value">
      <p class="timezone">Owner timezone: {{ dailyViewQuery.data.value.timezone }}</p>
      <DailySummary :daily-view="dailyViewQuery.data.value" />
      <p v-if="dailyViewQuery.data.value.summary.point_count === 0" class="empty-state">Không có dữ liệu GPS cho ngày này.</p>
      <RouteMap
        v-else
        :key="`${dailyViewQuery.data.value.device_id}:${dailyViewQuery.data.value.date}`"
        :daily-view="dailyViewQuery.data.value"
      />
    </template>
  </section>
</template>
