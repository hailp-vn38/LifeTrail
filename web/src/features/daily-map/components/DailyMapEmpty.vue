<script setup lang="ts">
import { MapPinOff } from "lucide-vue-next";
import AppEmptyState from "../../../components/ui/AppEmptyState.vue";

defineEmits<{ "go-today": [] }>();
</script>

<template>
  <AppEmptyState
    :icon="MapPinOff"
    title="Không có dữ liệu GPS cho ngày này"
    description="Device chưa ghi nhận vị trí nào trong ngày đã chọn. Hãy thử một ngày khác hoặc quay về hôm nay."
    action-label="Về hôm nay"
    @action="$emit('go-today')"
  />
</template>
