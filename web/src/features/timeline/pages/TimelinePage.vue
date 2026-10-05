<script setup lang="ts">
import { Clock } from "lucide-vue-next";
import PlaceholderPage from "../../placeholder/PlaceholderPage.vue";
</script>

<template>
  <PlaceholderPage
    title="Timeline"
    :icon="Clock"
    description="Timeline tổng hợp theo ngày — gồm trip, stop, ảnh và audio — sẽ có ở Phase 2. Hiện tại, mỗi ngày đã có timeline Start/End trong trang Daily Map."
  />
</template>
