<script setup lang="ts">
import { ChevronLeft, ChevronRight } from "lucide-vue-next";
import AppButton from "../../../components/ui/AppButton.vue";
import AppIconButton from "../../../components/ui/AppIconButton.vue";
import { formatDayLabel, shiftDay, todayForOwner } from "../../../lib/date";

const props = defineProps<{ date: string; timezone?: string }>();
const emit = defineEmits<{ "date-change": [date: string] }>();

function go(delta: number) {
  emit("date-change", shiftDay(props.date, delta));
}

function goToday() {
  emit("date-change", todayForOwner(props.timezone ?? "UTC"));
}
</script>

<template>
  <div class="daily-map-toolbar" role="toolbar" aria-label="Điều hướng ngày">
    <AppIconButton :icon="ChevronLeft" label="Ngày trước" @click="go(-1)" />
    <AppButton variant="ghost" size="sm" @click="goToday">Hôm nay</AppButton>
    <AppIconButton :icon="ChevronRight" label="Ngày sau" @click="go(1)" />
    <span class="daily-map-toolbar__label text-muted text-sm">{{ formatDayLabel(date) }}</span>
  </div>
</template>

<style scoped>
.daily-map-toolbar {
  display: flex;
  align-items: center;
  gap: 0.4rem;
}
.daily-map-toolbar__label {
  margin-left: 0.35rem;
}
</style>
