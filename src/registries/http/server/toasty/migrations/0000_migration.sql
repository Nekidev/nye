CREATE TYPE "arch" AS ENUM ('x86', 'x86_64', 'arm', 'aarch64', 'm68k', 'mips', 'mips32r6', 'mips64', 'mips64r6', 'csky', 'powerpc', 'powerpc64', 'riscv32', 'riscv64', 's390x', 'sparc', 'sparc64', 'hexagon', 'loongarch32', 'loongarch64');
CREATE TYPE "os" AS ENUM ('linux', 'windows', 'mac_os', 'android', 'ios', 'open_bsd', 'free_bsd', 'net_bsd', 'wasi', 'hermit', 'aix', 'apple', 'dragonfly', 'emscripten', 'espidf', 'fortanix', 'uefi', 'fuchsia', 'haiku', 'watch_os', 'vision_os', 'tv_os', 'horizon', 'hurd', 'illumos', 'l4re', 'nto', 'redox', 'solaris', 'solid_asp3', 'vexos', 'vita', 'vxworks', 'xous');
CREATE TABLE "package_versions" (
    "id" TEXT NOT NULL,
    "number" TEXT NOT NULL,
    "package_id" TEXT NOT NULL,
    "created_at" TEXT NOT NULL,
    "updated_at" TEXT NOT NULL,
    PRIMARY KEY ("id")
);
CREATE UNIQUE INDEX "index_package_versions_by_package_id_and_number" ON "package_versions" ("package_id", "number");
CREATE INDEX "index_package_versions_by_package_id" ON "package_versions" ("package_id");
CREATE TABLE "users" (
    "id" TEXT NOT NULL,
    "name" TEXT NOT NULL,
    "email" TEXT NOT NULL,
    "password" TEXT NOT NULL,
    "created_at" TEXT NOT NULL,
    "updated_at" TEXT NOT NULL,
    PRIMARY KEY ("id")
);
CREATE UNIQUE INDEX "index_users_by_name" ON "users" ("name");
CREATE UNIQUE INDEX "index_users_by_email" ON "users" ("email");
CREATE TABLE "packages" (
    "id" TEXT NOT NULL,
    "name" TEXT NOT NULL,
    "user_id" TEXT NOT NULL,
    "created_at" TEXT NOT NULL,
    "updated_at" TEXT NOT NULL,
    PRIMARY KEY ("id")
);
CREATE UNIQUE INDEX "index_packages_by_name" ON "packages" ("name");
CREATE INDEX "index_packages_by_user_id" ON "packages" ("user_id");
CREATE TABLE "package_version_bundles" (
    "id" TEXT NOT NULL,
    "target_os" os NOT NULL,
    "target_arch" arch NOT NULL,
    "file" TEXT NOT NULL,
    "version_id" TEXT NOT NULL,
    "created_at" TEXT NOT NULL,
    "updated_at" TEXT NOT NULL,
    PRIMARY KEY ("id")
);
CREATE INDEX "index_package_version_bundles_by_version_id" ON "package_version_bundles" ("version_id");
