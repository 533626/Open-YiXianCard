declare const __OPEN_YIXIAN_REPOSITORY_FIXTURES__: boolean | undefined;

/**
 * Development keeps the repository fixture catalog available. Production
 * static builds define this constant as false so bundlers can remove the
 * catalog-fetch branch and the UI presents local import only.
 */
export const repositoryFixtureCatalogEnabled =
  typeof __OPEN_YIXIAN_REPOSITORY_FIXTURES__ === "boolean"
    ? __OPEN_YIXIAN_REPOSITORY_FIXTURES__
    : true;
