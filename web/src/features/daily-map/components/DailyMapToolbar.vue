<script setup lang="ts">
import { computed } from "vue";
import { ChevronLeft, ChevronRight } from "lucide-vue-next";
import AppButton from "../../../components/ui/AppButton.vue";
import AppIconButton from "../../../components/ui/AppIconButton.vue";
import { weekCalendar } from "../lib/week-calendar";
import { formatDayLabel, shiftDay, todayForOwner } from "../../../lib/date";

const props = withDefaults(defineProps<{ date: string; timezone?: string; daysWithData?: string[] }>(), { daysWithData: () => [] });
const emit = defineEmits<{ "date-change": [date: string] }>();
const today = computed(() => todayForOwner(props.timezone ?? "UTC"));
const week = computed(() => weekCalendar(props.date));

function selectDate(date: string) {
  if (date && date !== props.date) emit("date-change", date);
}

const showToday = computed(() => !week.value.some(day => day.date === today.value));
</script>

<template>
  <section class="daily-map-toolbar" aria-label="Lịch tuần">
    <nav class="daily-map-toolbar__week" aria-label="Điều hướng ngày">
      <AppIconButton :icon="ChevronLeft" label="Tuần trước" @click="selectDate(shiftDay(date, -7))" />
      <div class="daily-map-toolbar__days">
        <button
          v-for="day in week"
          :key="day.date"
          type="button"
          class="daily-map-toolbar__day"
          :class="{ 'is-selected': day.date === date, 'is-today': day.date === today }"
          :aria-label="`${day.label}, ${formatDayLabel(day.date)}${day.date === today ? ' · Hôm nay' : ''}${daysWithData.includes(day.date) ? ' · Có dữ liệu GPS' : ''}`"
          :title="[day.date === today ? 'Hôm nay' : '', daysWithData.includes(day.date) ? 'Có dữ liệu GPS' : ''].filter(Boolean).join(' · ') || undefined"
          :aria-current="day.date === date ? 'date' : undefined"
          @click="selectDate(day.date)"
        >
          <span>{{ day.label }}</span>
          <strong>{{ day.day }}</strong>
          <span class="daily-map-toolbar__data-slot" aria-hidden="true"><span v-if="daysWithData.includes(day.date)" class="daily-map-toolbar__data-dot" /></span>
        </button>
      </div>
      <AppIconButton :icon="ChevronRight" label="Tuần sau" @click="selectDate(shiftDay(date, 7))" />
    </nav>
    <AppButton v-if="showToday" class="daily-map-toolbar__today" variant="ghost" size="sm" @click="selectDate(today)">Hôm nay</AppButton>
  </section>
</template>

<style scoped>
.daily-map-toolbar { position: relative; min-width: 0; }
.daily-map-toolbar__week { display: flex; align-items: center; gap: 8px; }
.daily-map-toolbar__week > :deep(button) { flex-shrink: 0; width: 32px; height: 32px; border-radius: 50%; border-color: var(--color-border); }
.daily-map-toolbar__days { display: grid; grid-template-columns: repeat(7, minmax(0, 1fr)); flex: 1; min-width: 0; gap: 4px; }
.daily-map-toolbar__day { position: relative; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 2px; height: 52px; padding: 4px; border: 0; border-radius: 12px; background: transparent; color: var(--color-text-muted); font-size: 11px; white-space: nowrap; }
.daily-map-toolbar__day strong { font-size: 18px; font-weight: 650; line-height: 1.25; color: var(--color-text); font-variant-numeric: tabular-nums; }
.daily-map-toolbar__data-slot { display: flex; justify-content: center; height: 5px; }
.daily-map-toolbar__data-dot { width: 4px; height: 4px; border-radius: 50%; background: var(--color-primary); }
.daily-map-toolbar__day:hover { background: #f5f8fc; }
.daily-map-toolbar__day.is-today { background: #ecfdf5; color: #047857; box-shadow: inset 0 0 0 1px #a7f3d0; }
.daily-map-toolbar__day.is-today strong { color: #047857; }
.daily-map-toolbar__day.is-selected { background: var(--color-primary-soft); color: var(--color-primary); }
.daily-map-toolbar__day.is-selected strong { color: var(--color-primary); }
.daily-map-toolbar__day.is-selected::after { content: ""; position: absolute; bottom: 0; width: 16px; height: 3px; border-radius: 3px; background: var(--color-primary); }
.daily-map-toolbar__day:focus-visible { outline: 2px solid var(--color-primary); outline-offset: 2px; }
.daily-map-toolbar__today { position: absolute; right: -4px; top: -7px; font-size: 10px; line-height: 1; padding: 0 4px; }
@media (max-width: 480px) {
  .daily-map-toolbar__week { gap: 4px; }
  .daily-map-toolbar__days { gap: 0; }
}
</style>
