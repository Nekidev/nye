CREATE TABLE "artifacts" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "package_name" TEXT NOT NULL,
    "version_number" TEXT NOT NULL,
    "path" TEXT NOT NULL,
    "kind" TEXT NOT NULL CHECK ("kind" IN ('bin', 'lib', 'env', 'var')),
    "link" TEXT NOT NULL,
    "created_at" INTEGER NOT NULL,
    "updated_at" INTEGER NOT NULL
);
-- #[toasty::breakpoint]
CREATE INDEX "index_artifacts_by_package_name_and_version_number" ON "artifacts" ("package_name", "version_number");
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_artifacts_by_kind_and_link" ON "artifacts" ("kind", "link");
-- #[toasty::breakpoint]
CREATE TABLE "packages" (
    "name" TEXT NOT NULL,
    "path" TEXT NOT NULL,
    "created_at" INTEGER NOT NULL,
    "updated_at" INTEGER NOT NULL,
    PRIMARY KEY ("name")
);
-- #[toasty::breakpoint]
CREATE TABLE "versions" (
    "number" TEXT NOT NULL,
    "package_name" TEXT NOT NULL,
    "path" TEXT NOT NULL,
    "created_at" INTEGER NOT NULL,
    "updated_at" INTEGER NOT NULL,
    PRIMARY KEY ("package_name", "number")
);
