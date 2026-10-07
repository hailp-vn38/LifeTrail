import { useQueries } from "@tanstack/vue-query";
import { computed, toValue, type MaybeRefOrGetter } from "vue";
import { getDailyView } from "../../../api/queries/daily-view.query";
import { weekCalendar } from "../lib/week-calendar";

/** Cache only availability; a dot represents actual Raw GPS, even before processing. */
export function useWeekData(
  deviceId: MaybeRefOrGetter<string>,
  date: MaybeRefOrGetter<string>,
  timezone: MaybeRefOrGetter<string | undefined>,
) {
  const week = computed(() => weekCalendar(toValue(date)));
  const queries = useQueries({
    queries: computed(() => {
      const id = toValue(deviceId);
      return week.value.map(day => ({
        queryKey: ["device", id, "day", day.date, "has-data", toValue(timezone)],
        queryFn: async () => {
          const view = await getDailyView(id, day.date, true);
          return view.summary.point_count > 0;
        },
        enabled: Boolean(id),
        staleTime: 60_000,
        retry: 1,
      }));
    }),
  });
  const daysWithData = computed(() => week.value.filter((_, index) => queries.value[index]?.data === true).map(day => day.date));

  function refresh() {
    return Promise.allSettled(queries.value.map(query => query.refetch()));
  }

  return { daysWithData, refresh };
}
