<script setup lang="ts">
import { computed } from "vue";
import PageHeader from "../../../components/layout/PageHeader.vue";
import AppCard from "../../../components/ui/AppCard.vue";
import { MAP_STYLE_PRESETS, mapStylePreset } from "../../../map/style-presets";
import { useMapPreferencesStore } from "../../../stores/map-preferences.store";

const preferences = useMapPreferencesStore();
const selectedStyle = computed({
  get: () => preferences.styleId,
  set: (value: string) => preferences.selectStyle(value),
});
const preset = computed(() => mapStylePreset(preferences.styleId));
const styleGroups = ["LifeTrail / MapTiler", "VersaTiles"].map((label) => ({
  label,
  options: MAP_STYLE_PRESETS.filter((option) => option.provider === label),
}));
</script>

<template>
  <div class="page settings-page">
    <PageHeader title="Settings" description="Tùy chỉnh cách hiển thị bản đồ của bạn." />
    <AppCard title="Bản đồ">
      <div class="map-settings">
        <label for="map-style">Kiểu bản đồ</label>
        <select id="map-style" v-model="selectedStyle" aria-describedby="map-style-description map-style-note">
          <optgroup v-for="group in styleGroups" :key="group.label" :label="group.label">
            <option v-for="option in group.options" :key="option.id" :value="option.id">
              {{ option.label }}
            </option>
          </optgroup>
        </select>
        <p id="map-style-description" class="text-muted text-sm">{{ preset.description }}</p>
        <p v-if="preset.pitch > 0" class="text-muted text-sm">
          Tòa nhà 3D hiện rõ khi phóng gần; độ chi tiết phụ thuộc dữ liệu của từng khu vực.
        </p>
        <p id="map-style-note" class="text-muted text-sm" role="status">
          {{ preferences.saved
            ? "Lựa chọn được lưu tự động trên trình duyệt này và áp dụng cho Daily Map."
            : "Đã áp dụng cho phiên hiện tại. Trình duyệt không cho phép lưu lựa chọn." }}
        </p>
      </div>
    </AppCard>
  </div>
</template>

<style scoped>
.settings-page { max-width: 48rem; }
.map-settings { display: flex; flex-direction: column; gap: 0.75rem; }
label { font-weight: 600; }
select {
  width: 100%;
  max-width: 24rem;
  padding: 0.75rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  background: var(--color-surface);
  color: var(--color-text);
  font: inherit;
}
select:focus-visible { outline: 2px solid var(--color-primary); outline-offset: 2px; }
</style>
