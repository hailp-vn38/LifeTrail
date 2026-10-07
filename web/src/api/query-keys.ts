export const queryKeys = {
  devices: ["devices"] as const,
  device: (deviceId: string) => ["device", deviceId] as const,
  dailyView: (deviceId: string, date: string, raw = false, projectionContext?: string) =>
    raw
      ? ["device", deviceId, "day", date, "raw", projectionContext] as const
      : ["device", deviceId, "day", date, projectionContext] as const,
  dailyStatus: (deviceId: string, date: string) => ["device", deviceId, "day", date, "status"] as const,
  playback: (deviceId: string, date: string, manifestVersion?: string) =>
    ["device", deviceId, "day", date, "playback", manifestVersion] as const,
  systemStatus: ["system", "status"] as const,
};
