import { computed, onMounted, onUnmounted, ref, watch, type Ref } from "vue";
import { ApiRequestError } from "../../../api/errors/api-error";
import { useDailyStatus, useDailyView } from "../../../api/queries/daily-view.query";

/** Daily Map data boundary: query + the states the page must render. */
export function useDailyMap(deviceId: Ref<string>, date: Ref<string>) {
  const rawMode = ref(false);
  const status = useDailyStatus(deviceId, date);
  const projectionContext = computed(() => {
    const current = status.data.value;
    return current ? `${current.timezone}:${current.timezone_generation}` : undefined;
  });
  const query = useDailyView(deviceId, date, rawMode, projectionContext);
  const publishedRevision = ref<string | null>(null);
  watch(() => query.data.value?.processing?.published_revision ?? null, (revision) => { publishedRevision.value = revision; }, { immediate: true });
  watch(() => status.data.value?.published_revision ?? null, async (revision) => {
    if (revision && publishedRevision.value && revision !== publishedRevision.value) await query.refetch();
  });
  const refreshStatus = () => status.refetch();
  onMounted(() => window.addEventListener("focus", refreshStatus));
  onMounted(() => document.addEventListener("visibilitychange", refreshStatus));
  onUnmounted(() => window.removeEventListener("focus", refreshStatus));
  onUnmounted(() => document.removeEventListener("visibilitychange", refreshStatus));

  const isNotFound = computed(
    () =>
      query.error.value instanceof ApiRequestError &&
      query.error.value.status === 404,
  );

  const isEmpty = computed(
    () => query.data.value != null && query.data.value.summary.point_count === 0 && !query.data.value.timeline?.length,
  );

  return { query, status, isNotFound, isEmpty, rawMode };
}
