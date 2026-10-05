<script setup lang="ts">
import { computed } from "vue";
import { RouterLink, useRoute, useRouter } from "vue-router";
import { ApiRequestError } from "../../../api/errors/api-error";
import { useDevice } from "../../../api/queries/devices.query";
import { todayForOwner } from "../../../lib/date";
import PageHeader from "../../../components/layout/PageHeader.vue";
import AppButton from "../../../components/ui/AppButton.vue";
import AppCard from "../../../components/ui/AppCard.vue";
import AppEmptyState from "../../../components/ui/AppEmptyState.vue";
import AppErrorState from "../../../components/ui/AppErrorState.vue";
import AppSkeleton from "../../../components/ui/AppSkeleton.vue";
import DeviceStatus from "../components/DeviceStatus.vue";

const route = useRoute();
const router = useRouter();
const deviceId = computed(() => String(route.params.deviceId));
const deviceQuery = useDevice(deviceId);

const isNotFound = computed(
  () =>
    deviceQuery.error.value instanceof ApiRequestError &&
    deviceQuery.error.value.status === 404,
);

const todayPath = computed(() => {
  const device = deviceQuery.data.value;
  if (!device) return null;
  return `/devices/${device.id}/day/${todayForOwner(device.timezone)}`;
});
</script>

<template>
  <div class="page">
    <PageHeader title="Device">
      <template #actions>
        <RouterLink to="/devices">← Devices</RouterLink>
      </template>
    </PageHeader>

    <AppSkeleton v-if="deviceQuery.isPending.value" :lines="4" />

    <AppErrorState
      v-else-if="isNotFound"
      title="Không tìm thấy Device"
      message="Device này không tồn tại hoặc đã bị xóa."
      retry-label="Về danh sách Devices"
      @retry="router.push('/devices')"
    />

    <AppErrorState
      v-else-if="deviceQuery.isError.value"
      title="Không thể tải Device"
      message="Đã có lỗi khi tải dữ liệu. Kiểm tra kết nối rồi thử lại."
      @retry="deviceQuery.refetch()"
    />

    <template v-else-if="deviceQuery.data.value">
      <AppCard :title="deviceQuery.data.value.name">
        <dl class="device-detail">
          <div><dt>Tên</dt><dd>{{ deviceQuery.data.value.name }}</dd></div>
          <div><dt>ID</dt><dd class="tabular">{{ deviceQuery.data.value.id }}</dd></div>
          <div><dt>Múi giờ</dt><dd>{{ deviceQuery.data.value.timezone }}</dd></div>
          <div><dt>Trạng thái</dt><dd><DeviceStatus /></dd></div>
        </dl>
        <div class="device-detail__actions">
          <RouterLink v-if="todayPath" :to="todayPath">
            <AppButton variant="primary">Mở Daily Map</AppButton>
          </RouterLink>
        </div>
      </AppCard>
      <AppEmptyState
        title="Lịch sử chi tiết đang phát triển"
        description="Phase 2 sẽ bổ sung trip, stop và timeline đầy đủ cho từng device tại đây."
      />
    </template>
  </div>
</template>

<style scoped>
.device-detail {
  display: grid;
  gap: 0.75rem;
  grid-template-columns: repeat(auto-fit, minmax(14rem, 1fr));
}
.device-detail > div {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
}
.device-detail dt {
  font-size: var(--font-size-xs);
  color: var(--color-text-muted);
}
.device-detail dd {
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
}
.device-detail__actions {
  margin-top: 1.25rem;
}
</style>
