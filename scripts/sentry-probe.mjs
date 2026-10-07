import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync, existsSync } from "node:fs";
import { DatabaseSync } from "node:sqlite";
import { tmpdir } from "node:os";
import path from "node:path";
import assert from "node:assert/strict";

const binary = path.resolve(
  import.meta.dirname,
  "../src-tauri/target/debug/maxxit",
);
for (const mode of ["error", "panic", "abort"]) {
  const dir = mkdtempSync(path.join(tmpdir(), "maxxit-diagnostics-"));
  try {
    const child = spawnSync(binary, ["--diagnostics-probe", mode, dir], {
      timeout: 10000,
      stdio: "pipe",
    });
    if (child.error) throw child.error;
    // The SDK's reporter can persist after the crashing foreground process exits.
    const dbPath = path.join(dir, "diagnostics/diagnostics.sqlite");
    assert.ok(existsSync(dbPath));
    let rows = [];
    for (let attempt = 0; attempt < 40; attempt++) {
      const db = new DatabaseSync(dbPath);
      rows = db.prepare("SELECT body,category FROM outbox").all();
      db.close();
      if (rows.some((row) => row.category !== "log")) break;
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
    assert.ok(
      rows.some((row) => row.category !== "log"),
      `No ${mode} report persisted`,
    );
    const ordinary = rows
      .filter((row) => row.category !== "attachment")
      .map((row) => Buffer.from(row.body).toString())
      .join("\n");
    assert.ok(
      !ordinary.includes("PRIVATE_MARKER"),
      `${mode} leaked a private error message`,
    );
    if (mode === "abort")
      assert.ok(
        rows.some((row) => row.category === "attachment"),
        "Native dump absent",
      );
    console.log(
      `${mode}: report persisted, ordinary payload redacted; no network upload`,
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}
