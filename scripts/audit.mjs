import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
const exceptions = JSON.parse(
  readFileSync(new URL("../docs/security-exceptions.json", import.meta.url)),
);
for (const exception of exceptions) {
  if (new Date(exception.expires) <= new Date())
    throw new Error("Expired advisory exception: " + exception.id);
  for (const target of ["aarch64-apple-darwin", "x86_64-apple-darwin"]) {
    const metadata = JSON.parse(
      execFileSync(
        "cargo",
        [
          "metadata",
          "--locked",
          "--format-version",
          "1",
          "--filter-platform",
          target,
          "--manifest-path",
          "src-tauri/Cargo.toml",
        ],
        { encoding: "utf8", maxBuffer: 16 * 1024 * 1024 },
      ),
    );
    const nodes = new Set(metadata.resolve.nodes.map((node) => node.id));
    if (
      metadata.packages.some(
        (pkg) => pkg.name === exception.crate && nodes.has(pkg.id),
      )
    )
      throw new Error(
        "Excepted crate is now compiled on " + target + ": " + exception.crate,
      );
  }
}
execFileSync(
  "cargo-audit",
  [
    "audit",
    "--file",
    "src-tauri/Cargo.lock",
    "--deny",
    "warnings",
    ...exceptions.flatMap((exception) => ["--ignore", exception.id]),
  ],
  { stdio: "inherit" },
);
