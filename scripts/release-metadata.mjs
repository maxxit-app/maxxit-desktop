import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { root, versions, newer } from "./version.mjs";
import {
  payloads,
  repository,
  validateManifest,
  verifyChecksums,
  archiveEntriesSafe,
  readArchiveEntries,
  changelogNotes,
} from "./release-lib.mjs";
const git = (...args) =>
  execFileSync("git", args, { cwd: root, encoding: "utf8" }).trim();
function preflight(version) {
  if (versions() !== version)
    throw new Error("Requested release version differs from source.");
  if (git("status", "--porcelain"))
    throw new Error("Release checkout must be clean.");
  if (git("branch", "--show-current") !== "main")
    throw new Error("Stable releases must use reviewed main.");
  const sha = git("rev-parse", "HEAD");
  const remote = git("ls-remote", "origin", "refs/heads/main").split(/\s/)[0];
  if (remote !== sha) throw new Error("Local main must match remote main.");
  const release = JSON.parse(
    execFileSync("gh", ["api", "repos/" + repository + "/releases/latest"], {
      encoding: "utf8",
    }),
  );
  if (!newer(version, release.tag_name.replace(/^v/, "")))
    throw new Error("Release version must exceed the current stable version.");
  const tag = git("ls-remote", "origin", "refs/tags/v" + version);
  if (tag)
    throw new Error("This release tag already exists. Use a new version.");
  const runs = JSON.parse(
    execFileSync(
      "gh",
      ["api", "repos/" + repository + "/actions/runs?head_sha=" + sha],
      { encoding: "utf8" },
    ),
  ).workflow_runs;
  for (const name of ["Check desktop", "Security"]) {
    const mainRuns = runs.filter((r) => r.name === name && r.event === "push");
    if (
      !mainRuns.length ||
      mainRuns[0].head_sha !== sha ||
      mainRuns[0].conclusion !== "success"
    )
      throw new Error(
        "Required main workflow has not passed for this revision: " + name,
      );
  }
  changelogNotes(
    readFileSync(path.join(root, "CHANGELOG.md"), "utf8"),
    version,
  );
  console.log(sha);
}
try {
  const [action, version, directory] = process.argv.slice(2);
  if (action === "preflight") preflight(version);
  else if (action === "manifest") {
    const signature = readFileSync(
      path.join(directory, payloads[2]),
      "utf8",
    ).trim();
    const artifact = {
      signature,
      url:
        "https://github.com/" +
        repository +
        "/releases/download/v" +
        version +
        "/" +
        payloads[1],
    };
    const manifest = {
      version,
      notes: changelogNotes(
        readFileSync(path.join(root, "CHANGELOG.md"), "utf8"),
        version,
      ),
      pub_date: new Date().toISOString(),
      platforms: { "darwin-aarch64": artifact, "darwin-x86_64": artifact },
    };
    validateManifest(manifest, version, signature);
    writeFileSync(
      path.join(directory, "latest.json"),
      JSON.stringify(manifest, null, 2) + "\n",
    );
  } else if (action === "verify") {
    if (versions() !== version && version !== "0.1.10")
      throw new Error("Version does not match the verification checkout.");
    verifyChecksums(directory, version === "0.1.10");
    validateManifest(
      JSON.parse(readFileSync(path.join(directory, "latest.json"), "utf8")),
      version,
      readFileSync(path.join(directory, payloads[2]), "utf8"),
    );
    const entries = readArchiveEntries(path.join(directory, payloads[1]));
    archiveEntriesSafe(entries);
    console.log("Release metadata, checksums and archive paths verified.");
  } else
    throw new Error(
      "Usage: release-metadata.mjs preflight|manifest|verify VERSION [DIRECTORY]",
    );
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
