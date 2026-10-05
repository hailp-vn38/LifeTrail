import { useQuery } from "@tanstack/vue-query";
import { computed, toValue, type MaybeRefOrGetter } from "vue";
import { getDailyView } from "../api/daily-views";
import { queryKeys } from "./keys";

export function useDailyView(
  deviceId: MaybeRefOrGetter<string>,
  date: MaybeRefOrGetter<string>,
) {
  return useQuery({
    queryKey: computed(() => queryKeys.dailyView(toValue(deviceId), toValue(date))),
    queryFn: () => getDailyView(toValue(deviceId), toValue(date)),
  });
}
