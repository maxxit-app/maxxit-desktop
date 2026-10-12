import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  readFileSync,
  writeFileSync,
  mkdirSync,
  readdirSync,
  copyFileSync,
  unlinkSync,
  rmSync,
} from "node:fs";
import path from "node:path";
import { SentryCli } from "@sentry/cli";

const root = path.resolve(import.meta.dirname, "..");
const version = JSON.parse(
  readFileSync(path.join(root, "package.json")),
).version;
const sha = execFileSync("git", ["rev-parse", "HEAD"], {
  cwd: root,
  encoding: "utf8",
}).trim();
const release = `maxxit-desktop@${version}+${sha}`;
const artifacts = path.join(root, "release-artifacts", "sentry", release);
const assets = path.join(root, "dist", "assets");
const hash = (file) =>
  createHash("sha256").update(readFileSync(file)).digest("hex");
const cli = new SentryCli();
const mode = process.argv[2];
if (mode === "prepare") {
  // Injection needs no account credentials. Run before Tauri embeds and signs these assets.
  await cli.execute(["sourcemaps", "inject", assets], true);
  rmSync(artifacts, { recursive: true, force: true });
  mkdirSync(artifacts, { recursive: true, mode: 0o700 });
  const files = {};
  for (const name of readdirSync(assets)) {
    if (!/\.(js|map)$/.test(name)) continue;
    files[name] = hash(path.join(assets, name));
    copyFileSync(path.join(assets, name), path.join(artifacts, name));
    if (name.endsWith(".map")) unlinkSync(path.join(assets, name));
  }
  if (!Object.keys(files).some((name) => name.endsWith(".map")))
    throw new Error("No source maps generated");
  writeFileSync(
    path.join(artifacts, "manifest.json"),
    JSON.stringify({ release, sha, version, files }, null, 2),
  );
} else if (mode === "upload") {
  for (const key of ["SENTRY_AUTH_TOKEN", "SENTRY_ORG", "SENTRY_PROJECT"])
    if (!process.env[key])
      throw new Error(`Set ${key} to upload release diagnostics`);
  const manifest = JSON.parse(
    readFileSync(path.join(artifacts, "manifest.json")),
  );
  if (manifest.release !== release)
    throw new Error("Diagnostic artifacts belong to a different release");
  for (const [name, digest] of Object.entries(manifest.files)) {
    if (hash(path.join(artifacts, name)) !== digest)
      throw new Error("Diagnostic artifact changed: " + name);
    if (name.endsWith(".js") && hash(path.join(assets, name)) !== digest)
      throw new Error("Rebuild Tauri with these exact frontend assets");
  }
  const binary = process.argv[3];
  if (
    !binary ||
    execFileSync(binary, ["--version"], { encoding: "utf8" }).trim() !==
      `maxxit ${version} ${sha}`
  )
    throw new Error("Provide the signed release binary for this revision");
  const symbols = path.join(artifacts, "maxxit.dSYM");
  execFileSync("dsymutil", [binary, "-o", symbols], { stdio: "inherit" });
  const uuids = (file) =>
    execFileSync("dwarfdump", ["--uuid", file], { encoding: "utf8" })
      .split("\n")
      .filter(Boolean)
      .map((line) =>
        line
          .match(/UUID: ([A-Fa-f0-9-]+) \(([^)]+)\)/)
          ?.slice(1)
          .join(":"),
      )
      .sort();
  const expected = uuids(binary);
  if (
    !expected.length ||
    JSON.stringify(expected) !== JSON.stringify(uuids(symbols))
  )
    throw new Error(
      "Native symbol identifiers do not match the shipped binary",
    );
  await cli.execute(
    [
      "debug-files",
      "check",
      path.join(symbols, "Contents/Resources/DWARF/maxxit"),
    ],
    true,
  );
  await cli.execute(["debug-files", "upload", "--wait", symbols], true);
  await cli.execute(
    ["sourcemaps", "upload", "--release", release, "--validate", artifacts],
    true,
  );
  await cli.execute(["releases", "new", release], true);
  await cli.execute(
    [
      "releases",
      "set-commits",
      release,
      "--commit",
      `maxxit-app/maxxit-desktop@${sha}`,
    ],
    true,
  );
  await cli.execute(["releases", "finalize", release], true);
} else
  throw new Error("Usage: sentry-artifacts.mjs prepare | upload SIGNED_BINARY");
