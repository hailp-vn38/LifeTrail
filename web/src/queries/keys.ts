export const queryKeys = {
  devices: ["devices"] as const,
  dailyView: (deviceId: string, date: string) =>
    ["device", deviceId, "day", date] as const,
};
