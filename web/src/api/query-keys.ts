export const queryKeys = {
  devices: ["devices"] as const,
  device: (deviceId: string) => ["device", deviceId] as const,
  dailyView: (deviceId: string, date: string) =>
    ["device", deviceId, "day", date] as const,
  systemStatus: ["system", "status"] as const,
};
