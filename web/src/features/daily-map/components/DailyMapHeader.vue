<script setup lang="ts">
import { RefreshCw } from "lucide-vue-next";
import { useRouter } from "vue-router";
import AppIconButton from "../../../components/ui/AppIconButton.vue";
import PageHeader from "../../../components/layout/PageHeader.vue";
import { useDevices } from "../../../api/queries/devices.query";
import { formatDayLabel, todayForOwner } from "../../../lib/date";

const props = defineProps<{ deviceId: string; date: string }>();
const emit = defineEmits<{ "date-change": [date: string]; refresh: [] }>();

const router = useRouter();
const devicesQuery = useDevices();

function onDeviceChange(event: Event) {
  const nextId = (event.target as HTMLSelectElement).value;
  const device = devicesQuery.data.value?.find((item) => item.id === nextId);
  if (!device || nextId === props.deviceId) return;
  router.push(`/devices/${nextId}/day/${todayForOwner(device.timezone)}`);
}

function onDateChange(event: Event) {
  const nextDate = (event.target as HTMLInputElement).value;
  if (nextDate && nextDate !== props.date) emit("date-change", nextDate);
}
</script>

<template>
  <PageHeader title="Daily Map" :description="formatDayLabel(date)">
    <template #actions>
      <label class="daily-map-header__field">
        <span class="text-muted text-sm">Device</span>
        <select
          :value="deviceId"
          :disabled="devicesQuery.isPending.value"
          aria-label="Chọn device"
          @change="onDeviceChange"
        >
          <option
            v-for="device in devicesQuery.data.value ?? []"
            :key="device.id"
            :value="device.id"
          >
            {{ device.name }}
          </option>
        </select>
      </label>
      <label class="daily-map-header__field">
        <span class="text-muted text-sm">Ngày</span>
        <input :value="date" type="date" aria-label="Chọn ngày" @change="onDateChange" />
      </label>
      <AppIconButton :icon="RefreshCw" label="Tải lại Daily Map" @click="emit('refresh')" />
    </template>
  </PageHeader>
</template>

<style scoped>
.daily-map-header__field {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}
.daily-map-header__field select,
.daily-map-header__field input {
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  background: var(--color-surface);
  padding: 0.45rem 0.6rem;
  font-size: var(--font-size-sm);
  max-width: 12rem;
}
</style>
