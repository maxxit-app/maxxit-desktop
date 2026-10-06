import { describe, expect, it } from "vitest";
import { tokenSeries, tokenSummary } from "./analytics";

describe("observed token analytics", () => {
  it("keeps unobserved days distinct from a recorded zero and uses UTC boundaries", () => {
    const series = tokenSeries(
      [{ startDate: "2026-10-05", tokens: 0 }],
      3,
      new Date("2026-10-06T23:30:00-04:00"),
    );
    expect(series).toEqual([
      { startDate: "2026-10-05", tokens: 0 },
      { startDate: "2026-10-06", tokens: null },
      { startDate: "2026-10-07", tokens: null },
    ]);
    expect(tokenSummary(series)).toMatchObject({
      total: 0,
      observedDays: 1,
      average: 0,
    });
  });
  it("calculates summaries only from the selected period and valid observations", () => {
    const series = tokenSeries(
      [
        { startDate: "2026-10-01", tokens: 1000 },
        { startDate: "2026-10-05", tokens: 20 },
        { startDate: "2026-10-05", tokens: 30 },
        { startDate: "2026-10-06", tokens: 10 },
        { startDate: "2026-10-06", tokens: -200 },
        { startDate: "2026-10-06", tokens: NaN },
      ],
      2,
      new Date("2026-10-06T12:00:00Z"),
    );
    expect(tokenSummary(series)).toEqual({
      total: 60,
      observedDays: 2,
      average: 30,
      peak: { startDate: "2026-10-05", tokens: 50 },
    });
    expect(tokenSummary(tokenSeries([], 7))).toMatchObject({
      average: null,
      peak: null,
      observedDays: 0,
    });
  });
});
