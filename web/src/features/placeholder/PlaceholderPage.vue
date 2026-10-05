<script setup lang="ts">
import { Construction } from "lucide-vue-next";
import type { Component } from "vue";
import AppEmptyState from "../../components/ui/AppEmptyState.vue";
import PageHeader from "../../components/layout/PageHeader.vue";

withDefaults(
  defineProps<{ title: string; description: string; icon?: Component }>(),
  { icon: undefined },
);
</script>

<template>
  <div class="page">
    <PageHeader :title="title" />
    <AppEmptyState
      :icon="icon ?? Construction"
      :title="`${title} đang được phát triển`"
      :description="description"
    />
  </div>
</template>
