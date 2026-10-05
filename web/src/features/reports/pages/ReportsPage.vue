<script setup lang="ts">
import { BarChart3 } from "lucide-vue-next";
import PlaceholderPage from "../../placeholder/PlaceholderPage.vue";
</script>

<template>
  <PlaceholderPage
    title="Reports"
    :icon="BarChart3"
    description="Báo cáo tổng hợp theo tuần/tháng (quãng đường, thời lượng di chuyển) sẽ được bổ sung khi server có API tổng hợp."
  />
</template>
