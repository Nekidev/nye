CREATE TABLE "exposed_bins" (
    "name" TEXT NOT NULL,
    "package_id" TEXT NOT NULL,
    "created_at" TEXT NOT NULL,
    "updated_at" TEXT NOT NULL,
    PRIMARY KEY ("name")
);
-- #[toasty::breakpoint]
CREATE INDEX "index_exposed_bins_by_package_id" ON "exposed_bins" ("package_id");
-- #[toasty::breakpoint]
CREATE TABLE "packages" (
    "name" TEXT NOT NULL,
    "version" TEXT NOT NULL,
    "created_at" TEXT NOT NULL,
    "updated_at" TEXT NOT NULL,
    PRIMARY KEY ("name")
);
