import { execFileSync } from "node:child_process";
import { versions } from "./version.mjs";
export function diagnostics({ version, system, architecture, channel }) {
  return {
    maxxitVersion: version,
    macOS: system,
    architecture,
    installation: channel,
  };
}
const channels = ["dmg", "homebrew", "script", "source"];
if (process.argv[1]?.endsWith("diagnostics.mjs")) {
  const channel = process.argv[2] ?? "source";
  if (!channels.includes(channel)) {
    console.error("Use dmg, homebrew, script or source.");
    process.exitCode = 1;
  } else {
    const system =
      process.platform === "darwin"
        ? execFileSync("sw_vers", ["-productVersion"], {
            encoding: "utf8",
          }).trim()
        : "not macOS";
    console.log(
      JSON.stringify(
        diagnostics({
          version: versions(),
          system,
          architecture: process.arch,
          channel,
        }),
        null,
        2,
      ),
    );
  }
}
