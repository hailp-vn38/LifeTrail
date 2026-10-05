<script setup lang="ts">
import { RouterLink } from "vue-router";
import { todayForOwner } from "../time/owner-date";
import { useDevices } from "../queries/use-devices";

const devicesQuery = useDevices();
</script>

<template>
  <section>
    <h1>Devices</h1>
    <p v-if="devicesQuery.isPending.value" role="status">Đang tải Devices…</p>
    <p v-else-if="devicesQuery.isError.value" role="alert">Không thể tải danh sách Devices.</p>
    <p v-else-if="!devicesQuery.data.value?.length">Chưa có Device nào.</p>
    <ul v-else class="device-list">
      <li v-for="device in devicesQuery.data.value" :key="device.id">
        <RouterLink :to="`/devices/${device.id}/day/${todayForOwner(device.timezone)}`">
          <strong>{{ device.name }}</strong>
          <span>{{ device.timezone }}</span>
        </RouterLink>
      </li>
    </ul>
  </section>
</template>
