<script setup lang="ts">
import { computed } from "vue";
import { useSystemStatus } from "../../api/queries/health.query";
import { version as appVersion } from "../../../package.json";

const statusQuery = useSystemStatus();
const statusText = computed(() => {
  if (statusQuery.isPending.value) return "Đang kiểm tra…";
  if (statusQuery.data.value?.ok) return "Hệ thống online";
  return "Mất kết nối API";
});
const statusTone = computed(() => {
  if (statusQuery.isPending.value) return "muted";
  return statusQuery.data.value?.ok ? "success" : "danger";
});
</script>

<template>
  <footer class="sidebar-footer">
    <p class="sidebar-footer__status">
      <span
        class="sidebar-footer__dot"
        :class="`sidebar-footer__dot--${statusTone}`"
        aria-hidden="true"
      />
      <span class="sidebar-footer__status-text">{{ statusText }}</span>
    </p>
    <p class="sidebar-footer__version text-muted">LifeTrail Web v{{ appVersion }}</p>
  </footer>
</template>

<style scoped>
.sidebar-footer {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  padding: 0.75rem 0.5rem 0.25rem;
  border-top: 1px solid var(--color-border);
  font-size: var(--font-size-xs);
}
.sidebar-footer__status {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-weight: 600;
}
.sidebar-footer__dot {
  width: 0.55rem;
  height: 0.55rem;
  border-radius: 999px;
  flex: none;
}
.sidebar-footer__dot--success {
  background: var(--color-success);
}
.sidebar-footer__dot--danger {
  background: var(--color-danger);
}
.sidebar-footer__dot--muted {
  background: #b6bfcc;
}
.sidebar-footer__status-text,
.sidebar-footer__version {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.sidebar-collapsed .sidebar-footer__status-text,
.sidebar-collapsed .sidebar-footer__version {
  display: none;
}
@media (max-width: 1279px) {
  .sidebar-footer__status-text,
  .sidebar-footer__version {
    display: none;
  }
}
</style>
