import fs from "node:fs";

const expectedVersion = process.argv[2];
const versions = {
  "package.json": JSON.parse(fs.readFileSync("package.json", "utf8")).version,
  "src-tauri/tauri.conf.json": JSON.parse(
    fs.readFileSync("src-tauri/tauri.conf.json", "utf8"),
  ).version,
  "src-tauri/Cargo.toml": fs
    .readFileSync("src-tauri/Cargo.toml", "utf8")
    .match(/^version\s*=\s*"([^"]+)"/m)?.[1],
};

const uniqueVersions = new Set(Object.values(versions));
if (uniqueVersions.size !== 1) {
  throw new Error(`Version mismatch: ${JSON.stringify(versions)}`);
}
if (expectedVersion && !uniqueVersions.has(expectedVersion)) {
  throw new Error(`Application version does not match tag version ${expectedVersion}`);
}

const stableAsset = "Shelfy_universal-apple-darwin.app.zip";
const stableUrl = `https://github.com/mcxen/shelfy/releases/latest/download/${stableAsset}`;
const updater = fs.readFileSync("src-tauri/src/updater.rs", "utf8");
const cask = fs.readFileSync("Casks/shelfy.rb", "utf8");
const workflow = fs.readFileSync(".github/workflows/build.yml", "utf8");

for (const [file, source] of [
  ["src-tauri/src/updater.rs", updater],
  ["Casks/shelfy.rb", cask],
]) {
  if (!source.includes(stableAsset) || !source.includes(stableUrl)) {
    throw new Error(`${file} does not use the stable macOS release contract`);
  }
}
if (
  !workflow.includes(stableAsset) ||
  !workflow.includes("https://github.com/mcxen/shelfy/releases/latest/download/$STABLE")
) {
  throw new Error(".github/workflows/build.yml does not publish the stable macOS release contract");
}

console.log(`Release contract OK: v${[...uniqueVersions][0]} -> ${stableUrl}`);
