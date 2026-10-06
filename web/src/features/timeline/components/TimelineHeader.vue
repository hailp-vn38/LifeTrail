<script setup lang="ts">
import { computed } from "vue";
import { X } from "lucide-vue-next";
import AppIconButton from "../../../components/ui/AppIconButton.vue";

const props = defineProps<{ eventCount: number; hasSelection: boolean; date?: string }>();
const dayLabel = computed(() => props.date ? new Intl.DateTimeFormat("vi-VN", { weekday: "long", day: "numeric", month: "short", timeZone: "UTC" }).format(new Date(`${props.date}T00:00:00Z`)) : "");
defineEmits<{ clear: [] }>();
</script>

<template>
  <div class="timeline-header">
    <h2 class="section-title">TIMELINE<span v-if="dayLabel"> · {{ dayLabel }}</span></h2>
    <AppIconButton
      :icon="X"
      label="Bỏ chọn sự kiện"
      variant="ghost"
      @click="$emit('clear')"
    />

  </div>
</template>

<style scoped>
.timeline-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
}
.timeline-header { flex: none; }
.section-title { font-size: .85rem; }
.section-title span { text-transform: none; font-weight: 500; }
</style>
