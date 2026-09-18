DROP INDEX "index_exposed_bins_by_package_id";
-- #[toasty::breakpoint]
ALTER TABLE "exposed_bins" RENAME COLUMN "package_id" TO "package_name";
-- #[toasty::breakpoint]
CREATE INDEX "index_exposed_bins_by_package_name" ON "exposed_bins" ("package_name");
