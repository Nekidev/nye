CREATE TABLE "exposed_libs" (
    "name" TEXT NOT NULL,
    "package_name" TEXT NOT NULL,
    "created_at" TEXT NOT NULL,
    "updated_at" TEXT NOT NULL,
    PRIMARY KEY ("name")
);
-- #[toasty::breakpoint]
CREATE INDEX "index_exposed_libs_by_package_name" ON "exposed_libs" ("package_name");
