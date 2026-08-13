export const RELEASE_META_NAMES = {
  steamBuild: "open-yixiancard:steam-build",
  ruleset: "open-yixiancard:ruleset",
  appCommit: "open-yixiancard:app-commit",
} as const;

export interface ReleaseMetadata {
  readonly bound: boolean;
  readonly steamBuild: string | null;
  readonly ruleset: string | null;
  readonly appCommit: string | null;
  readonly label: string;
  readonly detail: string;
}

type MetadataDocument = Pick<Document, "querySelector">;

export function readReleaseMetadata(root?: MetadataDocument | null): ReleaseMetadata {
  const documentRoot = root ?? (typeof document === "undefined" ? null : document);
  const steamBuild = metaContent(documentRoot, RELEASE_META_NAMES.steamBuild);
  const ruleset = metaContent(documentRoot, RELEASE_META_NAMES.ruleset);
  const appCommit = metaContent(documentRoot, RELEASE_META_NAMES.appCommit);
  const bound = Boolean(steamBuild && ruleset && appCommit);
  if (!bound) {
    return {
      bound: false,
      steamBuild,
      ruleset,
      appCommit,
      label: "本地开发 · 未绑定发布快照",
      detail: "缺少完整的 build / ruleset / app commit 元数据；不会猜测发布身份",
    };
  }
  const shortCommit = appCommit!.slice(0, 10);
  return {
    bound: true,
    steamBuild,
    ruleset,
    appCommit,
    label: `Build ${steamBuild} · Rules ${ruleset} · ${shortCommit}`,
    detail: `Steam build ${steamBuild} · ruleset ${ruleset} · app commit ${appCommit}`,
  };
}

function metaContent(root: MetadataDocument | null, name: string): string | null {
  const content = root
    ?.querySelector<HTMLMetaElement>(`meta[name="${name}"]`)
    ?.content
    .trim();
  return content ? content : null;
}
