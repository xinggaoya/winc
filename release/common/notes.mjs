// The release notes: the commits between the previous tag and this one, grouped by type.
//
// The notes come straight from the commit log. The subject of every commit is one line
// ("type: text" by the convention in CONTRIBUTING.md); the type decides the group the line is
// listed under.
import { execFileSync } from "node:child_process";
import { pathToFileURL } from "node:url";

// The commit types and the headings they are listed under, in the order they appear.
const GROUPS = [
  ["feat", "Features"],
  ["fix", "Fixes"],
  ["perf", "Performance"],
  ["refactor", "Refactoring"],
  ["docs", "Documentation"],
  ["build", "Build and packaging"],
  ["ci", "Continuous integration"],
  ["chore", "Chores"],
  ["style", "Style"],
  ["test", "Tests"],
  ["revert", "Reverts"],
];

// The subject of a conventional commit: the type, an optional scope, and the text. A commit that
// does not follow the convention is listed as it is.
const SUBJECT = /^([a-z]+)(?:\([^()]*\))?!?: (.+)$/;

function git(args) {
  return execFileSync("git", args, { encoding: "utf8", maxBuffer: 16 * 1024 * 1024 });
}

// The tag before `tag`; none when this is the first tag of the repository.
export function previousTag(tag) {
  try {
    return git(["describe", "--tags", "--abbrev=0", `${tag}^`]).trim() || null;
  } catch {
    return null;
  }
}

// The subjects of the commits between the previous tag and `tag`, oldest first.
export function commits(tag) {
  const from = previousTag(tag);
  const range = from ? `${from}..${tag}` : tag;
  return git(["log", "--no-merges", "--reverse", "--format=%s", range])
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean);
}

// The release notes as Markdown: a heading per group, one bullet per commit.
export function notes(tag) {
  const subjects = commits(tag);
  if (!subjects.length) return "";
  const groups = new Map(GROUPS.map(([type]) => [type, []]));
  const other = [];
  for (const subject of subjects) {
    const match = SUBJECT.exec(subject);
    const type = match ? match[1] : "";
    const text = match ? match[2] : subject;
    if (groups.has(type)) groups.get(type).push(text);
    else other.push(text);
  }
  const lines = [];
  for (const [type, heading] of GROUPS) {
    const items = groups.get(type);
    if (!items.length) continue;
    lines.push(`### ${heading}`, ...items.map((item) => `- ${item}`), "");
  }
  if (other.length) lines.push("### Other changes", ...other.map((item) => `- ${item}`), "");
  return lines.join("\n");
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const tag = process.argv[2];
  if (!tag) {
    console.error("usage: node release/common/notes.mjs <tag>");
    process.exitCode = 1;
  } else {
    console.log(notes(tag));
  }
}
