import {
  lstat,
  readFile,
} from "node:fs/promises";
import {
  join,
  resolve,
} from "node:path";

const DEFAULT_REPO_ROOT = resolve(import.meta.dir, "..");
const MIN_POLICY_CHARACTERS = 80;
const PLACEHOLDER_CONTENT = /(?:\b(?:TODO|TBD|FIXME|PLACEHOLDER|undecided)\b|to be determined|not (?:yet )?decided|pending (?:owner )?decision|coming soon|未决定|尚未决定|待定|待补|占位)/i;

export const PUBLIC_RELEASE_POLICY_FILES = [
  "LICENSE",
  "NOTICE",
  "CORPUS_POLICY.md",
] as const;

export interface PublicReleasePolicyAudit {
  readonly repoRoot: string;
  readonly files: readonly {
    readonly path: typeof PUBLIC_RELEASE_POLICY_FILES[number];
    readonly characters: number;
  }[];
}

export class PublicReleasePolicyError extends Error {
  constructor(readonly failures: readonly string[]) {
    super(`public release policy rejected:\n${failures.map((failure) => `- ${failure}`).join("\n")}`);
    this.name = "PublicReleasePolicyError";
  }
}

export async function checkPublicReleasePolicy(
  repoRoot = DEFAULT_REPO_ROOT,
): Promise<PublicReleasePolicyAudit> {
  const root = resolve(repoRoot);
  const failures: string[] = [];
  const files: PublicReleasePolicyAudit["files"][number][] = [];

  for (const path of PUBLIC_RELEASE_POLICY_FILES) {
    const absolutePath = join(root, path);
    const stat = await lstat(absolutePath).catch((error: NodeJS.ErrnoException) => {
      if (error.code === "ENOENT") return null;
      throw error;
    });
    if (stat === null) {
      failures.push(`missing required repository-root policy file: ${path}`);
      continue;
    }
    if (stat.isSymbolicLink() || !stat.isFile()) {
      failures.push(`policy path must be a regular non-symlink file: ${path}`);
      continue;
    }

    const bytes = await readFile(absolutePath);
    let content: string;
    try {
      content = new TextDecoder("utf-8", { fatal: true }).decode(bytes).trim();
    } catch {
      failures.push(`policy file must be UTF-8 text: ${path}`);
      continue;
    }
    if (content.length < MIN_POLICY_CHARACTERS) {
      failures.push(`policy file is too short to be a substantive decision: ${path}`);
      continue;
    }
    if (PLACEHOLDER_CONTENT.test(content)) {
      failures.push(`policy file still contains placeholder language: ${path}`);
      continue;
    }
    if (
      path === "CORPUS_POLICY.md" &&
      !/(?:corpus|fixture|replay|语料|夹具|回放)/i.test(content)
    ) {
      failures.push("CORPUS_POLICY.md does not identify the governed engineering corpus");
      continue;
    }
    if (
      path === "CORPUS_POLICY.md" &&
      !/(?:publish|public|distribut|redistribut|公开|发布|分发|再分发)/i.test(content)
    ) {
      failures.push("CORPUS_POLICY.md does not state a public distribution decision");
      continue;
    }
    files.push({ path, characters: content.length });
  }

  if (failures.length > 0) throw new PublicReleasePolicyError(failures);
  return { repoRoot: root, files };
}

if (import.meta.main) {
  try {
    if (process.argv.length > 3) {
      throw new Error("usage: bun scripts/check-public-release-policy.ts [repository-root]");
    }
    const audit = await checkPublicReleasePolicy(process.argv[2]);
    console.log(JSON.stringify({
      status: "public release policy accepted",
      repoRoot: audit.repoRoot,
      files: audit.files,
    }, null, 2));
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}
