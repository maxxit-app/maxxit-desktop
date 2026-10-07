import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";
export const root = fileURLToPath(new URL("../", import.meta.url));
export function stableVersion(value) {
  if (!/^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$/.test(value))
    throw new Error("Use a stable semantic version.");
  const parts = value.split(".").map(Number);
  if (!parts.every(Number.isSafeInteger))
    throw new Error("Version exceeds the supported range.");
  return parts;
}
export function newer(version, previous) {
  const a = stableVersion(version),
    b = stableVersion(previous);
  for (let i = 0; i < 3; i++) if (a[i] !== b[i]) return a[i] > b[i];
  return false;
}
export function versions(directory = root) {
  const read = (name) => readFileSync(path.join(directory, name), "utf8");
  const pkg = JSON.parse(read("package.json"));
  const tauri = JSON.parse(read("src-tauri/tauri.conf.json"));
  const cargo = read("src-tauri/Cargo.toml").match(
    /^version = "([^"]+)"$/m,
  )?.[1];
  const lock = read("src-tauri/Cargo.lock").match(
    /name = "maxxit"\nversion = "([^"]+)"/,
  )?.[1];
  const found = [pkg.version, tauri.version, cargo, lock];
  stableVersion(pkg.version);
  if (!found.every((v) => v === pkg.version))
    throw new Error(
      "Version mismatch across package, Tauri, Cargo and lockfile.",
    );
  return pkg.version;
}
export function bump(version, directory = root) {
  stableVersion(version);
  const current = versions(directory);
  if (!newer(version, current)) throw new Error("The version must increase.");
  for (const name of ["package.json", "src-tauri/tauri.conf.json"]) {
    const file = path.join(directory, name);
    const data = JSON.parse(readFileSync(file, "utf8"));
    data.version = version;
    writeFileSync(file, JSON.stringify(data, null, 2) + "\n");
  }
  for (const name of ["src-tauri/Cargo.toml", "src-tauri/Cargo.lock"]) {
    const file = path.join(directory, name);
    const text = readFileSync(file, "utf8");
    const pattern = name.endsWith(".toml")
      ? /^version = "[^"]+"$/m
      : /(name = "maxxit"\n)version = "[^"]+"/;
    writeFileSync(
      file,
      text.replace(
        pattern,
        (...args) =>
          (name.endsWith(".toml") ? "" : args[1]) +
          'version = "' +
          version +
          '"',
      ),
    );
  }
  return versions(directory);
}
if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  try {
    const action = process.argv[2];
    if (action === "bump") console.log(bump(process.argv[3]));
    else if (action === "check")
      console.log("Version agreement: " + versions());
    else throw new Error("Usage: version.mjs check | bump VERSION");
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
