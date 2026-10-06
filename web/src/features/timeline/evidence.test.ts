import { describe, expect, it } from "vitest";
import { qualityGapView } from "../../test/fixtures/quality-gaps";
import { describeEvidenceHoles, describeGap } from "./evidence";

const view = qualityGapView();
const gap = view.timeline?.[0];

describe("describeGap", () => {
  it("explains an absent observation interval without inventing travel", () => {
    expect(gap?.kind).toBe("gap");
    const described = describeGap(
      gap as NonNullable<typeof gap> & { kind: "gap" },
      view.timezone,
    );

    expect(described.title).toBe("GPS Gap");
    expect(described.subtitle).toContain("Thiếu quan sát GPS");
    expect(described.subtitle).toContain("11 phút");
    expect(described.subtitle).toContain("Không suy ra di chuyển");
    // A Gap carries no location, so the Timeline never centres on one.
    expect(described).not.toHaveProperty("coordinate");
  });
});

describe("describeEvidenceHoles", () => {
  it("reports each hole with its own reason and Raw record count", () => {
    const lines = describeEvidenceHoles(view);

    expect(lines).toHaveLength(2);
    expect(lines[0]).toContain("Có GPS nhưng vị trí không hợp lý");
    expect(lines[0]).toContain("7 bản ghi GPS");
    expect(lines[1]).toContain("Có GPS nhưng chất lượng chưa đủ");
    expect(lines[1]).toContain("5 bản ghi GPS");
  });

  it("never describes a hole as absent observations", () => {
    for (const line of describeEvidenceHoles(view)) {
      expect(line).not.toContain("Thiếu quan sát GPS");
    }
  });

  it("reports nothing when the day has no unresolved coverage", () => {
    expect(
      describeEvidenceHoles({ evidence_holes: [], timezone: view.timezone }),
    ).toEqual([]);
  });
});