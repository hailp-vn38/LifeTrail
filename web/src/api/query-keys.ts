export const queryKeys = {
  devices: ["devices"] as const,
  device: (deviceId: string) => ["device", deviceId] as const,
  dailyView: (deviceId: string, date: string, raw = false) =>
    raw ? ["device", deviceId, "day", date, "raw"] as const : ["device", deviceId, "day", date] as const,
  dailyStatus: (deviceId: string, date: string) => ["device", deviceId, "day", date, "status"] as const,
  systemStatus: ["system", "status"] as const,
};
