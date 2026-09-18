ALTER TABLE "packages" ADD COLUMN "location" TEXT NOT NULL;
-- #[toasty::breakpoint]
ALTER TABLE "exposed_bins" ADD COLUMN "location" TEXT NOT NULL;
-- #[toasty::breakpoint]
ALTER TABLE "exposed_libs" ADD COLUMN "location" TEXT NOT NULL;
