import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import path from "node:path";
import { stableVersion } from "./version.mjs";
export const repository = "maxxit-app/maxxit-desktop";
export const team = "DXYF58SJPA";
export const identifier = "app.maxxit.desktop";
export const payloads = [
  "Maxxit-universal.dmg",
  "Maxxit-universal.app.tar.gz",
  "Maxxit-universal.app.tar.gz.sig",
  "latest.json",
];
export function validateManifest(manifest, version, signature) {
  stableVersion(version);
  if (
    manifest.version !== version ||
    !Number.isFinite(Date.parse(manifest.pub_date)) ||
    !manifest.notes?.trim()
  )
    throw new Error("Invalid updater version, date or notes.");
  if (
    Object.keys(manifest.platforms ?? {})
      .sort()
      .join(",") !== "darwin-aarch64,darwin-x86_64"
  )
    throw new Error("Both supported updater targets are required.");
  for (const target of Object.values(manifest.platforms)) {
    const expected =
      "https://github.com/" +
      repository +
      "/releases/download/v" +
      version +
      "/Maxxit-universal.app.tar.gz";
    if (target.url !== expected || target.signature !== signature.trim())
      throw new Error("Updater target URL or signature mismatch.");
  }
}
export function verifyChecksums(directory, legacy = false) {
  const manifest = readFileSync(path.join(directory, "SHA256SUMS"), "utf8");
  const entries = new Map();
  for (const line of manifest.trim().split("\n")) {
    const match = line.match(/^([a-f0-9]{64}) {2}([A-Za-z0-9._-]+)$/);
    if (!match || entries.has(match[2]) || !payloads.includes(match[2]))
      throw new Error("Invalid or duplicate checksum entry.");
    entries.set(match[2], match[1]);
  }
  const required = legacy ? payloads.slice(0, 2) : payloads;
  for (const name of required) {
    const actual = createHash("sha256")
      .update(readFileSync(path.join(directory, name)))
      .digest("hex");
    if (entries.get(name) !== actual)
      throw new Error("Missing or invalid checksum for " + name);
  }
}
// BSD tar hides AppleDouble records when listing. Inspect the actual member names.
export function readArchiveEntries(archive) {
  return JSON.parse(
    execFileSync(
      "python3",
      [
        "-c",
        "import json,sys,tarfile; archive=tarfile.open(sys.argv[1], 'r:gz'); print(json.dumps([m.name + '/' if m.isdir() and not m.name.endswith('/') else m.name for m in archive.getmembers()]))",
        archive,
      ],
      { encoding: "utf8" },
    ),
  );
}
export function archiveEntriesSafe(entries) {
  if (
    !entries.length ||
    entries.some(
      (p) =>
        !p.startsWith("Maxxit.app/") ||
        p.split("/").includes("..") ||
        p.split("/").some((part) => part.startsWith("._")) ||
        p.startsWith("/") ||
        p.includes("\\"),
    )
  )
    throw new Error("Unsafe updater archive paths.");
}
export function changelogNotes(text, version) {
  const marker = "## " + version + " ";
  const start = text.indexOf(marker);
  if (start < 0)
    throw new Error("Add a dated changelog section for this version.");
  const lines = text.slice(start).split("\n");
  const stop = lines.findIndex(
    (line, index) => index > 0 && line.startsWith("## "),
  );
  const notes = lines
    .slice(1, stop < 0 ? undefined : stop)
    .join("\n")
    .trim();
  if (!notes || /\b(?:TODO|TBD)\b/.test(notes))
    throw new Error("Finish the release notes before packaging.");
  return notes;
}
