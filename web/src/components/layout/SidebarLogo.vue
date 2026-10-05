<template>
  <RouterLink to="/" class="sidebar-logo" aria-label="LifeTrail home">
    <img src="/logo.svg" alt="" width="32" height="32" aria-hidden="true" />
    <span class="sidebar-logo__wordmark">LifeTrail</span>
  </RouterLink>
</template>

<style scoped>
.sidebar-logo {
  display: flex;
  align-items: center;
  gap: 0.65rem;
  padding: 0.25rem 0.5rem;
  border-radius: var(--radius-sm);
  color: var(--color-text);
}
.sidebar-logo:hover {
  text-decoration: none;
}
.sidebar-logo__wordmark {
  font-size: var(--font-size-lg);
  font-weight: 700;
  letter-spacing: -0.01em;
  white-space: nowrap;
}
.sidebar-collapsed .sidebar-logo__wordmark {
  display: none;
}
@media (max-width: 1279px) {
  .sidebar-logo__wordmark {
    display: none;
  }
}
@media (max-width: 767px) {
  .sidebar-logo__wordmark {
    display: inline;
  }
}
</style>
