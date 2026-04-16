import { mkdtempSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { tmpdir } from "node:os";
import { join } from "node:path";

const targetDir = mkdtempSync(join(tmpdir(), "codexlag-cargo-target-"));
const cargoBinary = process.platform === "win32" ? "cargo.exe" : "cargo";
const extraArgs = process.argv.slice(2);
const result = spawnSync(
  cargoBinary,
  ["test", "--manifest-path", "src-tauri/Cargo.toml", ...extraArgs],
  {
    cwd: process.cwd(),
    stdio: "inherit",
    env: {
      ...process.env,
      CARGO_TARGET_DIR: targetDir,
    },
  },
);

if (result.error) {
  throw result.error;
}

process.exit(result.status ?? 1);
