<script setup lang="ts">
import { Cpu, Map } from "lucide-vue-next";
import { RouterLink } from "vue-router";
import type { Device } from "../../../api/queries/devices.query";
import { todayForOwner } from "../../../lib/date";
import AppCard from "../../../components/ui/AppCard.vue";
import DeviceStatus from "./DeviceStatus.vue";

defineProps<{ device: Device }>();
</script>

<template>
  <AppCard>
    <div class="device-card">
      <span class="device-card__icon" aria-hidden="true"><Cpu :size="22" /></span>
      <div class="device-card__body">
        <h3 class="device-card__name">{{ device.name }}</h3>
        <p class="device-card__meta text-muted text-sm">{{ device.timezone }}</p>
        <p class="device-card__meta text-muted text-sm tabular">{{ device.id }}</p>
      </div>
      <DeviceStatus />
    </div>
    <div class="device-card__actions">
      <RouterLink
        class="device-card__link"
        :to="`/devices/${device.id}/day/${todayForOwner(device.timezone)}`"
      >
        <Map :size="16" aria-hidden="true" /> Daily Map hôm nay
      </RouterLink>
      <RouterLink class="device-card__link" :to="`/devices/${device.id}`">Chi tiết</RouterLink>
    </div>
  </AppCard>
</template>

<style scoped>
.device-card {
  display: flex;
  align-items: flex-start;
  gap: 0.9rem;
}
.device-card__icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 2.75rem;
  height: 2.75rem;
  border-radius: var(--radius-sm);
  background: var(--color-primary-soft);
  color: var(--color-primary);
  flex: none;
}
.device-card__body {
  flex: 1;
  min-width: 0;
}
.device-card__name {
  font-size: var(--font-size-md);
  font-weight: 650;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.device-card__meta {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.device-card__actions {
  display: flex;
  gap: 1rem;
  margin-top: 1rem;
  padding-top: 0.9rem;
  border-top: 1px solid var(--color-border);
}
.device-card__link {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  font-size: var(--font-size-sm);
  font-weight: 600;
}
</style>
