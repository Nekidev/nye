CREATE TYPE "token_kind" AS ENUM ('access', 'refresh');
CREATE TABLE "users" (
    "id" TEXT NOT NULL,
    "name" TEXT NOT NULL,
    "email" TEXT NOT NULL,
    "password" TEXT NOT NULL,
    "created_at" BIGINT NOT NULL,
    "updated_at" BIGINT NOT NULL,
    PRIMARY KEY ("id")
);
CREATE UNIQUE INDEX "index_users_by_name" ON "users" ("name");
CREATE UNIQUE INDEX "index_users_by_email" ON "users" ("email");
CREATE TABLE "package_version_bundles" (
    "id" TEXT NOT NULL,
    "target" TEXT NOT NULL,
    "file" TEXT NOT NULL,
    "version_id" TEXT NOT NULL,
    "created_at" BIGINT NOT NULL,
    "updated_at" BIGINT NOT NULL,
    PRIMARY KEY ("id")
);
CREATE UNIQUE INDEX "index_package_version_bundles_by_version_id_and_target" ON "package_version_bundles" ("version_id", "target");
CREATE INDEX "index_package_version_bundles_by_version_id" ON "package_version_bundles" ("version_id");
CREATE TABLE "tokens" (
    "id" TEXT NOT NULL,
    "kind" token_kind NOT NULL,
    "user_id" TEXT NOT NULL,
    "created_at" BIGINT NOT NULL,
    "updated_at" BIGINT NOT NULL,
    "expires_at" BIGINT NOT NULL,
    PRIMARY KEY ("id")
);
CREATE INDEX "index_tokens_by_user_id" ON "tokens" ("user_id");
CREATE TABLE "package_versions" (
    "id" TEXT NOT NULL,
    "number" TEXT NOT NULL,
    "package_id" TEXT NOT NULL,
    "created_at" BIGINT NOT NULL,
    "updated_at" BIGINT NOT NULL,
    PRIMARY KEY ("id")
);
CREATE UNIQUE INDEX "index_package_versions_by_package_id_and_number" ON "package_versions" ("package_id", "number");
CREATE INDEX "index_package_versions_by_package_id" ON "package_versions" ("package_id");
CREATE TABLE "packages" (
    "id" TEXT NOT NULL,
    "name" TEXT NOT NULL,
    "user_id" TEXT NOT NULL,
    "created_at" BIGINT NOT NULL,
    "updated_at" BIGINT NOT NULL,
    PRIMARY KEY ("id")
);
CREATE UNIQUE INDEX "index_packages_by_name" ON "packages" ("name");
CREATE INDEX "index_packages_by_user_id" ON "packages" ("user_id");
