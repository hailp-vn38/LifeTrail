<script setup lang="ts">
import PageHeader from "../../../components/layout/PageHeader.vue";
import AppEmptyState from "../../../components/ui/AppEmptyState.vue";
import AppErrorState from "../../../components/ui/AppErrorState.vue";
import AppSkeleton from "../../../components/ui/AppSkeleton.vue";
import { useDevices } from "../../../api/queries/devices.query";
import DeviceCard from "../components/DeviceCard.vue";

const devicesQuery = useDevices();
</script>

<template>
  <div class="page">
    <PageHeader
      title="Devices"
      description="Các thiết bị đang đồng bộ dữ liệu GPS về LifeTrail."
    />

    <AppSkeleton v-if="devicesQuery.isPending.value" :lines="4" />

    <AppErrorState
      v-else-if="devicesQuery.isError.value"
      title="Không thể tải danh sách Devices"
      message="Đã có lỗi khi tải dữ liệu. Kiểm tra kết nối rồi thử lại."
      @retry="devicesQuery.refetch()"
    />

    <AppEmptyState
      v-else-if="!devicesQuery.data.value?.length"
      title="Chưa có Device nào"
      description="Đăng ký một device qua CLI provisioning để bắt đầu đồng bộ GPS."
    />

    <div v-else class="device-list">
      <DeviceCard
        v-for="device in devicesQuery.data.value"
        :key="device.id"
        :device="device"
      />
    </div>
  </div>
</template>

<style scoped>
.device-list {
  display: grid;
  gap: 1rem;
  grid-template-columns: repeat(auto-fill, minmax(20rem, 1fr));
}
</style>
