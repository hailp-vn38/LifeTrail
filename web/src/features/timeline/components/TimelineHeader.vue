<script setup lang="ts">
import { X } from "lucide-vue-next";
import AppIconButton from "../../../components/ui/AppIconButton.vue";

defineProps<{ eventCount: number; hasSelection: boolean }>();
defineEmits<{ clear: [] }>();
</script>

<template>
  <div class="timeline-header">
    <h2 class="section-title">Timeline</h2>
    <AppIconButton
      v-if="hasSelection"
      :icon="X"
      label="Bỏ chọn sự kiện"
      variant="ghost"
      @click="$emit('clear')"
    />
    <span v-else class="text-muted text-sm tabular">{{ eventCount }} sự kiện</span>
  </div>
</template>

<style scoped>
.timeline-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
}
</style>
