const PRIVATE_FIXTURE_DEPENDENCY = /(?:^|[\\/])(?:battle-evaluator[\\/]fixtures|fixtures[\\/](?:candidates|incoming|contracts))(?:[\\/]|$)/i;
const REPOSITORY_CATALOG_DEPENDENCY = /(?:^|[\\/])(?:fixture-index\.json|repository-replay-loader(?:\.ts)?)(?:$|[?#])/i;

export interface PublicBundleBoundaryOptions {
  readonly forbidRepositoryCatalog?: boolean;
  readonly allowedVirtualNamespaces?: readonly string[];
  readonly virtualCatalogReplacements?: {
    readonly fixtureIndex: string;
    readonly repositoryLoader: string;
  };
}

export function publicBundleBoundaryPlugin(
  options: PublicBundleBoundaryOptions = {},
): Bun.BunPlugin {
  return {
    name: "public-bundle-boundary",
    setup(build) {
      build.onResolve({ filter: /.*/ }, (args) => {
        assertPublicBundleDependencyPath(args.path, "import", options);
        return undefined;
      });
    },
  };
}

export function assertPublicBundleMetafile(
  metafile: Bun.BuildMetafile | undefined,
  label: string,
  options: PublicBundleBoundaryOptions = {},
): void {
  if (!metafile) throw new Error(`${label} build did not produce the required dependency metafile`);
  for (const [inputPath, input] of Object.entries(metafile.inputs)) {
    if (!isAllowedVirtualDependency(inputPath, options)) {
      assertPublicBundleDependencyPath(inputPath, `${label} metafile input`, options);
    }
    for (const dependency of input.imports) {
      if (isAllowedVirtualDependency(dependency.path, options)) continue;
      if (isReplacedCatalogDependency(dependency.path, metafile, options)) continue;
      assertPublicBundleDependencyPath(dependency.path, `${label} metafile import`, options);
      if (dependency.original) {
        if (isReplacedCatalogDependency(dependency.original, metafile, options)) continue;
        assertPublicBundleDependencyPath(dependency.original, `${label} original import`, options);
      }
    }
  }
}

function isReplacedCatalogDependency(
  path: string,
  metafile: Bun.BuildMetafile,
  options: PublicBundleBoundaryOptions,
): boolean {
  const replacements = options.virtualCatalogReplacements;
  if (!replacements) return false;
  if (/(?:^|[\\/])fixture-index\.json(?:$|[?#])/i.test(path)) {
    return replacements.fixtureIndex in metafile.inputs;
  }
  if (/(?:^|[\\/])repository-replay-loader(?:\.ts)?(?:$|[?#])/i.test(path)) {
    return replacements.repositoryLoader in metafile.inputs;
  }
  return false;
}

function isAllowedVirtualDependency(
  path: string,
  options: PublicBundleBoundaryOptions,
): boolean {
  return (options.allowedVirtualNamespaces ?? [])
    .some((namespace) => path.startsWith(`${namespace}:`));
}

export function assertPublicBundleDependencyPath(
  path: string,
  source: string,
  options: PublicBundleBoundaryOptions = {},
): void {
  if (PRIVATE_FIXTURE_DEPENDENCY.test(path)) {
    throw new Error(`${source} depends on private fixture content: ${path}`);
  }
  if (options.forbidRepositoryCatalog && REPOSITORY_CATALOG_DEPENDENCY.test(path)) {
    throw new Error(`${source} depends on a development-only replay catalog module: ${path}`);
  }
}
