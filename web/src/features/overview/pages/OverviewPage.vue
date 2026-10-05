<script setup lang="ts">
import { computed } from "vue";
import { RouterLink } from "vue-router";
import { Activity, Cpu, Map } from "lucide-vue-next";
import PageHeader from "../../../components/layout/PageHeader.vue";
import AppBadge from "../../../components/ui/AppBadge.vue";
import AppCard from "../../../components/ui/AppCard.vue";
import AppEmptyState from "../../../components/ui/AppEmptyState.vue";
import AppSkeleton from "../../../components/ui/AppSkeleton.vue";
import { useDevices } from "../../../api/queries/devices.query";
import { useSystemStatus } from "../../../api/queries/health.query";
import { todayForOwner } from "../../../lib/date";

const statusQuery = useSystemStatus();
const devicesQuery = useDevices();

const deviceCount = computed(() => devicesQuery.data.value?.length ?? 0);
const apiBadge = computed(() => {
  if (statusQuery.isPending.value) return { variant: "muted" as const, text: "Đang kiểm tra…" };
  if (statusQuery.data.value?.ok) return { variant: "success" as const, text: "API online" };
  return { variant: "danger" as const, text: "API offline" };
});
</script>

<template>
  <div class="page">
    <PageHeader title="Overview" description="Tổng quan hệ thống LifeTrail." />

    <div class="overview-grid">
      <AppCard title="Trạng thái hệ thống">
        <div class="overview-stat">
          <Activity :size="22" aria-hidden="true" />
          <div>
            <p><AppBadge :variant="apiBadge.variant">{{ apiBadge.text }}</AppBadge></p>
            <p class="text-muted text-sm">API qua reverse proxy Nginx, cùng origin với Web.</p>
          </div>
        </div>
      </AppCard>
      <AppCard title="Devices">
        <div class="overview-stat">
          <Cpu :size="22" aria-hidden="true" />
          <div>
            <p class="overview-stat__value tabular">{{ deviceCount }}</p>
            <p class="text-muted text-sm">thiết bị đã đăng ký</p>
          </div>
        </div>
        <p class="overview-card__link"><RouterLink to="/devices">Quản lý Devices →</RouterLink></p>
      </AppCard>
    </div>

    <AppCard title="Daily Map hôm nay">
      <AppSkeleton v-if="devicesQuery.isPending.value" :lines="3" />
      <AppEmptyState
        v-else-if="!devicesQuery.data.value?.length"
        title="Chưa có Device nào"
        description="Đăng ký một device để xem Daily Map."
      />
      <ul v-else class="overview-devices">
        <li v-for="device in devicesQuery.data.value" :key="device.id">
          <RouterLink
            :to="`/devices/${device.id}/day/${todayForOwner(device.timezone)}`"
            class="overview-devices__link"
          >
            <Map :size="18" aria-hidden="true" />
            <span>
              <strong>{{ device.name }}</strong>
              <span class="text-muted text-sm"> · {{ device.timezone }}</span>
            </span>
          </RouterLink>
        </li>
      </ul>
    </AppCard>
  </div>
</template>

<style scoped>
.overview-grid {
  display: grid;
  gap: 1rem;
  grid-template-columns: repeat(auto-fit, minmax(16rem, 1fr));
}
.overview-stat {
  display: flex;
  gap: 0.9rem;
  align-items: flex-start;
  color: var(--color-primary);
}
.overview-stat__value {
  font-size: var(--font-size-2xl);
  font-weight: 700;
  color: var(--color-text);
  line-height: 1;
}
.overview-stat p {
  color: var(--color-text);
}
.overview-card__link {
  margin-top: 0.9rem;
  font-size: var(--font-size-sm);
  font-weight: 600;
}
.overview-devices {
  list-style: none;
  display: flex;
  flex-direction: column;
}
.overview-devices li + li {
  border-top: 1px solid var(--color-border);
}
.overview-devices__link {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.8rem 0.25rem;
  color: var(--color-text);
}
.overview-devices__link:hover {
  text-decoration: none;
  color: var(--color-primary);
}
</style>
