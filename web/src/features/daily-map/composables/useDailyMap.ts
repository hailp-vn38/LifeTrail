import { computed, type Ref } from "vue";
import { ApiRequestError } from "../../../api/errors/api-error";
import { useDailyView } from "../../../api/queries/daily-view.query";

/** Daily Map data boundary: query + the states the page must render. */
export function useDailyMap(deviceId: Ref<string>, date: Ref<string>) {
  const query = useDailyView(deviceId, date);

  const isNotFound = computed(
    () =>
      query.error.value instanceof ApiRequestError &&
      query.error.value.status === 404,
  );

  const isEmpty = computed(
    () => query.data.value != null && query.data.value.summary.point_count === 0,
  );

  return { query, isNotFound, isEmpty };
}
