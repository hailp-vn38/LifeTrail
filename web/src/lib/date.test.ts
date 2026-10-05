import { describe, expect, it } from "vitest";
import { todayForOwner } from "./date";

describe("todayForOwner", () => {
  it("uses the Owner timezone rather than the browser timezone", () => {
    const instant = new Date("2026-10-04T00:30:00.000Z");

    expect(todayForOwner("America/Los_Angeles", instant)).toBe("2026-10-03");
    expect(todayForOwner("Asia/Ho_Chi_Minh", instant)).toBe("2026-10-04");
  });
});
