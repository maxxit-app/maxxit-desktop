import { describe, expect, it } from "vitest";
import { safeAttributes, safeBrowserEvent } from "./observability";

describe("diagnostic privacy boundary", () => {
  it("drops content and unrecognized metadata", () => {
    expect(
      safeAttributes({
        provider: "codex",
        stage: "read",
        status: 503,
        token: "PRIVATE",
        command: "PRIVATE",
        route: "/secret?token=PRIVATE",
        bytes: -1,
        target_version: "PRIVATE",
      }),
    ).toEqual({ provider: "codex", stage: "read", status: 503 });
  });
  it("preserves positions and Debug IDs without error messages, paths or props", () => {
    const safe = safeBrowserEvent({
      type: undefined,
      event_id: "a".repeat(32),
      message: "PRIVATE",
      user: { email: "PRIVATE" },
      extra: { props: "PRIVATE" },
      exception: {
        values: [
          {
            type: "TypeError",
            value: "PRIVATE",
            stacktrace: {
              frames: [
                {
                  filename: "/Users/PRIVATE/index-a.js?token=PRIVATE",
                  function: "render",
                  lineno: 4,
                  colno: 2,
                  context_line: "PRIVATE",
                  vars: { token: "PRIVATE" },
                },
              ],
            },
          },
        ],
      },
      debug_meta: {
        images: [
          {
            type: "sourcemap",
            code_file: "/Users/PRIVATE/index-a.js",
            debug_id: "a1234567-1234-1234-1234-123456789abc",
          },
        ],
      },
    });
    expect(JSON.stringify(safe)).not.toContain("PRIVATE");
    expect(safe.exception.values?.[0].stacktrace.frames?.[0]).toMatchObject({
      filename: "app:///assets/index-a.js",
      lineno: 4,
      colno: 2,
    });
    expect(safe.debug_meta.images?.[0]).toMatchObject({
      code_file: "app:///assets/index-a.js",
      debug_id: "a1234567-1234-1234-1234-123456789abc",
    });
  });
});
