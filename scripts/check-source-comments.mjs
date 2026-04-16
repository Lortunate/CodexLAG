import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const ROOTS = ["src", "src-tauri/src"];
const SOURCE_FILE_PATTERN = /\.(ts|tsx|rs)$/;
const offenders = [];

function normalizePath(path) {
  return path.replaceAll("\\", "/");
}

function recordOffense(filePath, lineNumber) {
  offenders.push(`${normalizePath(relative(process.cwd(), filePath))}:${lineNumber}`);
}

function scanFile(filePath) {
  const lines = readFileSync(filePath, "utf8").split(/\r?\n/);
  let inBlockComment = false;

  lines.forEach((line, index) => {
    const trimmed = line.trimStart();
    const hasNonAscii = /[^\x00-\x7F]/.test(line);

    if (inBlockComment) {
      if (hasNonAscii) {
        recordOffense(filePath, index + 1);
      }
      if (trimmed.includes("*/")) {
        inBlockComment = false;
      }
      return;
    }

    if (trimmed.startsWith("//")) {
      if (hasNonAscii) {
        recordOffense(filePath, index + 1);
      }
      return;
    }

    if (trimmed.startsWith("/*") || trimmed.startsWith("/**")) {
      if (hasNonAscii) {
        recordOffense(filePath, index + 1);
      }
      if (!trimmed.includes("*/")) {
        inBlockComment = true;
      }
      return;
    }

    if (trimmed.startsWith("*")) {
      if (hasNonAscii) {
        recordOffense(filePath, index + 1);
      }
    }
  });
}

function walk(directory) {
  for (const entry of readdirSync(directory)) {
    const fullPath = join(directory, entry);
    const stat = statSync(fullPath);

    if (stat.isDirectory()) {
      walk(fullPath);
      continue;
    }

    if (SOURCE_FILE_PATTERN.test(fullPath)) {
      scanFile(fullPath);
    }
  }
}

ROOTS.forEach(walk);

if (offenders.length) {
  console.error("Non-ASCII source comments found:");
  offenders.forEach((offender) => console.error(`- ${offender}`));
  process.exit(1);
}
