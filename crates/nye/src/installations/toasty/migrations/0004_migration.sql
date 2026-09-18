DROP TABLE "exposed_bins";
-- #[toasty::breakpoint]
DROP TABLE "exposed_libs";
-- #[toasty::breakpoint]
CREATE TABLE "exposed_artifacts" (
    "id" TEXT NOT NULL,
    "name" TEXT NOT NULL,
    "kind" TEXT NOT NULL CHECK ("kind" IN ('binary', 'library', 'variable')),
    "location" TEXT NOT NULL,
    "package_name" TEXT NOT NULL,
    "created_at" TEXT NOT NULL,
    "updated_at" TEXT NOT NULL,
    PRIMARY KEY ("id")
);
-- #[toasty::breakpoint]
CREATE INDEX "index_exposed_artifacts_by_kind" ON "exposed_artifacts" ("kind");
-- #[toasty::breakpoint]
CREATE INDEX "index_exposed_artifacts_by_package_name" ON "exposed_artifacts" ("package_name");
