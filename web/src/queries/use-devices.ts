import { useQuery } from "@tanstack/vue-query";
import { listDevices } from "../api/devices";
import { queryKeys } from "./keys";

export function useDevices() {
  return useQuery({ queryKey: queryKeys.devices, queryFn: listDevices });
}
