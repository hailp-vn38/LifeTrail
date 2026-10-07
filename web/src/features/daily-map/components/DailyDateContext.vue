<script setup lang="ts">
import { computed, ref } from "vue";
import { CalendarDays, ChevronDown } from "lucide-vue-next";

const props = defineProps<{ date: string }>();
const emit = defineEmits<{ "date-change": [date: string] }>();
const picker = ref<HTMLDetailsElement>();
const fullDate = computed(() => {
  const instant = new Date(`${props.date}T00:00:00Z`);
  if (Number.isNaN(instant.getTime())) return props.date;
  const weekday = instant.getUTCDay();
  const label = weekday === 0 ? "Chủ nhật" : `Thứ ${weekday + 1}`;
  return `${label}, ${props.date.slice(8)} tháng ${props.date.slice(5, 7)}, ${props.date.slice(0, 4)}`;
});

function selectDate(event: Event) {
  const date = (event.target as HTMLInputElement).value;
  if (!date) return;
  if (date !== props.date) emit("date-change", date);
  if (picker.value) picker.value.open = false;
}
</script>

<template>
  <div class="daily-date-context">
    <details ref="picker" class="daily-date-context__picker" @keydown.esc="picker && (picker.open = false)">
      <summary aria-label="Mở lịch chọn ngày">
        <CalendarDays :size="18" aria-hidden="true" />
        <span>{{ fullDate }}</span>
        <ChevronDown :size="14" aria-hidden="true" />
      </summary>
      <div class="daily-date-context__popover">
        <label for="daily-date-picker">Chọn ngày</label>
        <input id="daily-date-picker" :value="date" type="date" aria-label="Chọn ngày" @change="selectDate" />
      </div>
    </details>
    <p>7 ngày <span aria-hidden="true">•</span> 00:00 – 23:59</p>
  </div>
</template>

<style scoped>
.daily-date-context { min-width: 0; }
.daily-date-context__picker { position: relative; }
.daily-date-context summary { display: flex; align-items: center; gap: 8px; list-style: none; cursor: pointer; font-size: 13px; font-weight: 600; white-space: nowrap; }
.daily-date-context summary::-webkit-details-marker { display: none; }
.daily-date-context summary > svg { flex-shrink: 0; color: var(--color-text-muted); }
.daily-date-context summary:hover { color: var(--color-primary); }
.daily-date-context p { margin-top: 8px; padding-left: 26px; color: var(--color-text-muted); font-size: 11px; }
.daily-date-context p span { padding: 0 4px; }
.daily-date-context__popover { position: absolute; top: calc(100% + 12px); left: 0; z-index: 20; display: grid; gap: 8px; width: 240px; padding: 16px; border: 1px solid var(--color-border); border-radius: 12px; background: var(--color-surface); box-shadow: var(--shadow-popover); font-size: 12px; }
.daily-date-context input { width: 100%; padding: 8px; border: 1px solid var(--color-border); border-radius: 8px; background: var(--color-surface); }
</style>
