<template>
  <span class="app-shimmer-block" aria-hidden="true" />
</template>

<style scoped>
.app-shimmer-block {
  display: block;
  position: relative;
  min-height: 12px;
  overflow: hidden;
  border-radius: 8px;
  background: #edf1f7;
}
.app-shimmer-block::after {
  content: "";
  position: absolute;
  inset: 0;
  background: linear-gradient(90deg, transparent, rgb(255 255 255 / 65%), transparent);
  transform: translateX(-100%);
  animation: shimmer-block 1.6s ease-in-out infinite;
}
@keyframes shimmer-block {
  to { transform: translateX(100%); }
}
@media (prefers-reduced-motion: reduce) {
  .app-shimmer-block::after { animation: none; }
}
</style>
