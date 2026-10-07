import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
const npm = JSON.parse(
  execFileSync("pnpm", ["licenses", "list", "--prod", "--json"], {
    encoding: "utf8",
    maxBuffer: 16 * 1024 * 1024,
  }),
);
const rows = [];
for (const entries of Object.values(npm))
  for (const pkg of entries)
    for (const version of pkg.versions)
      rows.push([
        "npm",
        pkg.name,
        version,
        pkg.license ?? "Review required",
        pkg.homepage ?? "https://www.npmjs.com/package/" + pkg.name,
      ]);
const rust = new Map();
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
  for (const pkg of metadata.packages)
    if (pkg.source && nodes.has(pkg.id)) rust.set(pkg.id, pkg);
}
for (const pkg of rust.values())
  rows.push([
    "Cargo",
    pkg.name,
    pkg.version,
    pkg.license ?? "Review required",
    pkg.repository ?? "https://crates.io/crates/" + pkg.name,
  ]);
rows.sort((a, b) => a.join(" ").localeCompare(b.join(" "), "en"));
const escape = (value) => value.replaceAll("|", "\\|").replaceAll("\n", " ");
const output =
  "# Third-party dependency inventory\n\nGenerated from the locked production npm dependencies and both Mac Cargo compilation graphs.\nIncludes Rust build dependencies. Regenerate with pnpm licenses:generate after dependency changes.\nThe license identifiers below are dependency metadata; preserve upstream license and copyright notices when distributing code.\n\nManrope's [SIL Open Font License](public/fonts/OFL.txt) and [provider asset attribution](public/providers/ATTRIBUTION.md) are included separately.\nMaxxit's original code uses the [MIT license](LICENSE).\n\n| Ecosystem | Package | Version | License | Upstream |\n| --- | --- | --- | --- | --- |\n" +
  rows.map((row) => "| " + row.map(escape).join(" | ") + " |").join("\n") +
  "\n";
if (process.argv.includes("--check")) {
  if (readFileSync("THIRD_PARTY_NOTICES.md", "utf8") !== output)
    throw new Error(
      "Dependency inventory is stale. Run pnpm licenses:generate.",
    );
} else writeFileSync("THIRD_PARTY_NOTICES.md", output);
