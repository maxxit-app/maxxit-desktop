import { test } from "node:test";
import assert from "node:assert/strict";
import {
  mkdtempSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
  rmSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { createHash } from "node:crypto";
import { versions, bump, newer, stableVersion } from "./version.mjs";
import {
  payloads,
  validateManifest,
  verifyChecksums,
  archiveEntriesSafe,
} from "./release-lib.mjs";
import { diagnostics } from "./diagnostics.mjs";
import { parseDocument } from "yaml";

function fixture(t) {
  const root = mkdtempSync(path.join(tmpdir(), "maxxit-tooling-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  return root;
}
test("version bump updates all sources and rejects mismatches and downgrades", (t) => {
  const root = fixture(t);
  mkdirSync(path.join(root, "src-tauri"));
  writeFileSync(path.join(root, "package.json"), '{"version":"0.1.10"}');
  writeFileSync(
    path.join(root, "src-tauri/tauri.conf.json"),
    '{"version":"0.1.10"}',
  );
  writeFileSync(
    path.join(root, "src-tauri/Cargo.toml"),
    '[package]\nversion = "0.1.10"\n',
  );
  writeFileSync(
    path.join(root, "src-tauri/Cargo.lock"),
    '[[package]]\nname = "maxxit"\nversion = "0.1.10"\n',
  );
  assert.equal(bump("0.1.11", root), "0.1.11");
  assert.throws(() => bump("0.1.10", root));
  writeFileSync(path.join(root, "package.json"), '{"version":"0.1.12"}');
  assert.throws(() => versions(root));
  for (const version of ["1.2.3-beta", "1.2", "01.2.3", "1.2.3;echo bad"])
    assert.throws(() => stableVersion(version));
  assert.ok(newer("0.2.0", "0.1.99"));
});
test("updater metadata requires both targets and immutable authentic artifact references", () => {
  const target = {
    signature: "signed",
    url: "https://github.com/maxxit-app/maxxit-desktop/releases/download/v0.1.11/Maxxit-universal.app.tar.gz",
  };
  const manifest = {
    version: "0.1.11",
    pub_date: "2026-10-07T10:00:00Z",
    notes: "Fix",
    platforms: { "darwin-aarch64": target, "darwin-x86_64": target },
  };
  validateManifest(manifest, "0.1.11", "signed");
  assert.throws(() => validateManifest(manifest, "0.1.12", "signed"));
  assert.throws(() => validateManifest(manifest, "0.1.11", "other-key"));
  assert.throws(() =>
    validateManifest(
      { ...manifest, platforms: { "darwin-aarch64": target } },
      "0.1.11",
      "signed",
    ),
  );
  assert.throws(() =>
    validateManifest(
      {
        ...manifest,
        platforms: {
          ...manifest.platforms,
          "darwin-aarch64": { ...target, url: "http://evil.test/file" },
        },
      },
      "0.1.11",
      "signed",
    ),
  );
});
test("checksums reject missing files, duplicate entries and tampered bytes", (t) => {
  const root = fixture(t);
  const lines = payloads.map((name) => {
    writeFileSync(path.join(root, name), name);
    return createHash("sha256").update(name).digest("hex") + "  " + name;
  });
  writeFileSync(path.join(root, "SHA256SUMS"), lines.join("\n") + "\n");
  verifyChecksums(root);
  writeFileSync(path.join(root, "latest.json"), "tampered");
  assert.throws(() => verifyChecksums(root));
  writeFileSync(path.join(root, "SHA256SUMS"), lines.slice(0, 2).join("\n"));
  assert.throws(() => verifyChecksums(root));
  writeFileSync(
    path.join(root, "SHA256SUMS"),
    lines.concat(lines[0]).join("\n"),
  );
  assert.throws(() => verifyChecksums(root));
});
test("updater extraction rejects traversal and unexpected app names", () => {
  archiveEntriesSafe(["Maxxit.app/", "Maxxit.app/Contents/MacOS/maxxit"]);
  for (const entries of [
    [],
    ["/etc/passwd"],
    ["Maxxit.app/../../escape"],
    ["Other.app/Contents"],
  ])
    assert.throws(() => archiveEntriesSafe(entries));
});
test("diagnostics contain only allowlisted metadata", () => {
  const result = diagnostics({
    version: "0.1.10",
    system: "14.0",
    architecture: "arm64",
    channel: "dmg",
    token: "private",
    home: "/private/home",
    email: "private@example.test",
  });
  assert.deepEqual(Object.keys(result).sort(), [
    "architecture",
    "installation",
    "macOS",
    "maxxitVersion",
  ]);
  assert.ok(!JSON.stringify(result).includes("private"));
});
test("workflows are parseable, SHA-pinned and keep PR tokens read-only", () => {
  for (const name of ["check", "security"]) {
    const text = readFileSync(
      new URL("../.github/workflows/" + name + ".yml", import.meta.url),
      "utf8",
    );
    const document = parseDocument(text);
    assert.equal(document.errors.length, 0);
    const workflow = document.toJS();
    assert.equal(workflow.permissions.contents, "read");
    for (const job of Object.values(workflow.jobs))
      for (const step of job.steps ?? [])
        if (step.uses) assert.match(step.uses, /@[a-f0-9]{40}$/);
    assert.ok(!text.includes("pull_request_target"));
  }
});
test("tray permissions cannot mutate settings, credentials or install updates", () => {
  const main = JSON.parse(
    readFileSync(
      new URL("../src-tauri/capabilities/default.json", import.meta.url),
    ),
  );
  const tray = JSON.parse(
    readFileSync(
      new URL("../src-tauri/capabilities/tray.json", import.meta.url),
    ),
  );
  assert.ok(main.permissions.includes("allow-cloud-redeem"));
  assert.deepEqual(tray.permissions, [
    "core:event:default",
    "allow-snapshot",
    "allow-tray-action",
    "allow-tray-resize",
    "allow-diagnostics-record",
    "allow-diagnostics-health",
  ]);
  assert.ok(
    !tray.permissions.some((permission) =>
      /updater|cloud|settings|diagnostics-consent|diagnostics-export|diagnostics-test|diagnostics-flush/.test(
        permission,
      ),
    ),
  );
});
