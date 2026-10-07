import { shiftDay } from "../../../lib/date";

const weekdays = ["Thứ 2", "Thứ 3", "Thứ 4", "Thứ 5", "Thứ 6", "Thứ 7", "CN"];

export function weekCalendar(date: string) {
  const weekday = new Date(`${date}T00:00:00Z`).getUTCDay();
  const monday = shiftDay(date, -((weekday + 6) % 7));
  return weekdays.map((label, index) => {
    const date = shiftDay(monday, index);
    return { date, label, day: Number(date.slice(-2)) };
  });
}
