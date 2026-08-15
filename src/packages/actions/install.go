package actions

import (
	"archive/zip"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"

	"github.com/bmatcuk/doublestar/v4"
	"nyeki.dev/nye/packages"
	"nyeki.dev/nye/utils"
)

func InstallPackage(ctx packages.Context, path string) (packages.Manifest, error) {
	manifest, err := packages.GetManifestFromZip(path)
	if err != nil {
		return packages.Manifest{}, fmt.Errorf("could not get package manifest: %v", err)
	}

	if manifest.Package.Target != utils.GetCurrentTarget() {
		return packages.Manifest{}, fmt.Errorf("the package was bundled for the target %v, but your system is %v", manifest.Package.Target, utils.GetCurrentTarget())
	}

	collides, err := checkPackageVersionCollision(ctx, manifest)
	if err != nil {
		return packages.Manifest{}, fmt.Errorf("could not check for package installation collisions: %v", err)
	}
	if collides {
		return packages.Manifest{}, fmt.Errorf("the package's version is already installed")
	}

	msg, err := checkExposedBinCollisions(ctx, manifest)
	if err != nil {
		return packages.Manifest{}, fmt.Errorf("could not check for exposed binary collisions: %v", err)
	}
	if msg != "" {
		return packages.Manifest{}, errors.New(msg)
	}

	zipper, err := zip.OpenReader(path)
	if err != nil {
		return packages.Manifest{}, fmt.Errorf("could not reopen package file: %v", err)
	}
	defer zipper.Close()

	err = checkPackageZipSafety(zipper)
	if err != nil {
		return packages.Manifest{}, fmt.Errorf("the package file is unsafe: %v", err)
	}

	err = checkPackageStructure(zipper)
	if err != nil {
		return packages.Manifest{}, fmt.Errorf("the package file had an invalid internal structure: %v", err)
	}

	location := filepath.Join(ctx.Path, "pkg", "packages", manifest.Package.Name, manifest.Package.Version)
	err = extract(zipper, location, ctx)
	if err != nil {
		return packages.Manifest{}, fmt.Errorf("could not extract package file: %v", err)
	}

	err = exposeBinaries(manifest, ctx)
	if err != nil {
		return packages.Manifest{}, fmt.Errorf("could not expose one or more binaries: %v", err)
	}

	return manifest, nil
}

// Checks whether there's already a current installed package with the same name and version.
//
// Arguments:
// * `ctx` - The installation context.
// * `manifest` - The manifest to check collisions for.
//
// Returns:
// * `bool` - `true` if a collision exists, `false` if there's no collision or if an error occurred.
// * `error` - If an error occurred while checking for collisions.
func checkPackageVersionCollision(ctx packages.Context, manifest packages.Manifest) (bool, error) {
	installationDir := filepath.Join(ctx.Path, "pkg", "packages", manifest.Package.Name, manifest.Package.Version)
	exists, err := utils.Exists(installationDir)
	if err != nil {
		return false, fmt.Errorf("could not check if package collided with an existing installed package: %v", err)
	}

	return exists, nil
}

// Checks whether the manifest's exposed binaries collide with existing installed packages.
//
// Arguments:
// * `ctx` - The installation context.
// * `manifest` - The manifest to check collisions for.
//
// Returns:
// * `string` - If a collision exists, contains a user-intended message describing it. Empty otherwise.
// * `error` - If an error occurred while checking for collisions.
func checkExposedBinCollisions(ctx packages.Context, manifest packages.Manifest) (string, error) {
	for _, bin := range manifest.Exposes.Bin {
		exposedPath := filepath.Join(ctx.Path, "bin", bin.Name)
		exists, err := utils.Exists(exposedPath)
		if err != nil {
			return "", fmt.Errorf("could not check if exposed binary already existed: %v", err)
		}
		if exists {
			artifact, err := packages.GetArtifactMetadataFromPath(exposedPath)
			if err != nil {
				return "", fmt.Errorf("there's already an exposed binary `%v`. the package's metadata could not be retrieved: %v", bin.Name, err)
			}
			return fmt.Sprintf("there's already an exposed binary `%v` for package `%v %v`", bin.Name, artifact.Package.Name, artifact.Package.Version), nil
		}
	}

	return "", nil
}

// Ensures all file paths in the zip are safe to extract.
//
// Unsafe paths are:
// * Unclean paths, e.g. `../something`, `./something`, `something/../../else`, `some//thing`.
// * Paths that go through extracted symlinks.
//
// Paths that go through extracted symlinks are unsafe because they allow files to be extracted in
// arbitrary places. For example:
//
// * `bin/symlink -> /bin`
// * `bin/symlink/sh` will extract at `/bin/sh`
//
// Arguments:
// * `zipper` - The already-opened zip reader.
//
// Returns:
// * `error` - An error if the package is unsafe, `nil` otherwise.
func checkPackageZipSafety(zipper *zip.ReadCloser) error {
	symlinkPaths := []string{}

	for _, file := range zipper.File {
		info := file.FileInfo()

		if !utils.IsSafePath(info.Name()) {
			return fmt.Errorf("the package contains an unsafe path for a file, `%v`", info.Name())
		}

		if info.Mode()&os.ModeSymlink != 0 {
			symlinkPaths = append(symlinkPaths, info.Name())
		}
	}

	for _, file := range zipper.File {
		for _, symlinkPath := range symlinkPaths {
			if isParent(symlinkPath, file.Name) {
				return fmt.Errorf("the file `%v` was attempted to be written through symlink `%v`, which is unsafe", file.Name, symlinkPath)
			}
		}
	}

	return nil
}

// Checks whether a path is a parent directory of another path.
//
// This function does not check the actual type of the paths in the filesystem.
//
// Some examples:
// * `isParent("/etc", "/etc/hostname") == true`
// * `isParent("/etc/myconfig.txt", "/etc/myconfig.txt/something-else") == true`
// * `isParent("/etc/../pkg", "/pkg/packages") == true`
// * `isParent("something", "../something") == false`
// * `fmt.Println(isParent("a/b/c", "b/../b/c")) == false`
//
// Arguments:
// * `parent` - The base path.
// * `child` - The path to check whether it is located inside `parent`.
//
// Returns:
// * `true` - If `child` is in `parent`.
// * `false` - Otherwise.
func isParent(parent, child string) bool {
	parent = filepath.Clean(parent)
	child = filepath.Clean(child)

	// Rel calls filepath.Clean() on the resulting path before returning it.
	//
	// If child is a child of parent, the first segment will be anything but `..` or `.`. If it's
	// not a child, the result of filepath.Rel() will either be a path starting with `..` or `.` or
	// an error.
	//
	// Identical paths are not considered children of one another.
	rel, err := filepath.Rel(parent, child)
	if err != nil {
		return false
	}

	firstSegment := strings.SplitN(rel, "/", 2)[0]

	return firstSegment != ".." && firstSegment != "."
}

// Checks that the files in the package zip are structured correctly.
//
// Paths allowed:
// * `package.toml`
// * `bin/**/*`
//
// Note that absolute paths will not match. Packages are required to contain a `package.toml` file.
//
// Returns:
// * `error` - An error if a file is out of place, `nil` otherwise.
func checkPackageStructure(zipper *zip.ReadCloser) error {
	patterns := []string{
		"package.toml",
		"bin/**/*",
	}

	hasManifest := false

	for _, file := range zipper.File {
		matches := false

		for _, pattern := range patterns {
			match, err := doublestar.Match(pattern, file.Name)
			if err != nil {
				panic(fmt.Sprintf("An invalid GLOB `%v` was attempted to be matched: %v", pattern, err))
			}
			if match {
				matches = true
				break
			}
		}

		if file.Name == "package.toml" {
			hasManifest = true
		}

		if !matches {
			return fmt.Errorf("the package contained a file, `%v`, which was out of place", file.Name)
		}
	}

	if !hasManifest {
		return fmt.Errorf("the package file did not contain a manifest")
	}

	return nil
}

// Extracts a zip file into a directory.
//
// Arguments:
// * `zipper` - The zip reader to extract.
// * `location` - The directory to extract the zip file into.
// * `ctx` - The packages context.
func extract(zipper *zip.ReadCloser, location string, ctx packages.Context) error {
	for _, file := range zipper.File {
		info := file.FileInfo()

		if info.Mode()&os.ModeSymlink != 0 {
			err := extractSymlink(file, location, ctx)
			if err != nil {
				return fmt.Errorf("could not extract symlink from package: %v", err)
			}
		} else if info.IsDir() {
			err := extractDirectory(file, location, ctx)
			if err != nil {
				return fmt.Errorf("could not extract directory from package: %v", err)
			}
		} else {
			err := extractFile(file, location, ctx)
			if err != nil {
				return fmt.Errorf("could not extract file from package: %v", err)
			}
		}
	}

	return nil
}

// Extracts a symlink from a zip file and writes it to the system.
//
// You may want to use `extract()` instead, which extracts a full zip file.
//
// Note that `outputLocation` is not the final location of this symlink. The final location will be
// `filepath.Join(outputLocation, inputFile.Name)`.
//
// Arguments:
// * `inputFile` - The symlink file to extract.
// * `outputLocation` - The directory where the zip is being extracted.
// * `ctx` - The packages context.
func extractSymlink(inputFile *zip.File, outputLocation string, ctx packages.Context) error {
	path := filepath.Join(outputLocation, inputFile.Name)

	dir, _ := filepath.Split(path)
	err := os.MkdirAll(dir, ctx.DirPerms)
	if err != nil {
		return fmt.Errorf("could not create parent directories for symlink: %v", err)
	}

	reader, err := inputFile.Open()
	if err != nil {
		return fmt.Errorf("could not open symlink file in package: %v", err)
	}
	defer reader.Close()

	pointer, err := io.ReadAll(reader)
	if err != nil {
		return fmt.Errorf("could not read pointer of symlink in package: %v", err)
	}

	err = os.Symlink(string(pointer), path)
	if err != nil {
		return fmt.Errorf("could not export symlink from package: %v", err)
	}

	return nil
}

// Extracts a directory from a zip file and writes it to the system.
//
// You may want to use `extract()` instead, which extracts a full zip file.
//
// Note that `outputLocation` is not the final location of this directory. The final location will
// be `filepath.Join(outputLocation, inputFile.Name)`.
//
// This function does not extract any files and directories inside this directory, only creates
// this directory in the system.
//
// Arguments:
// * `inputFile` - The directory file to extract.
// * `outputLocation` - The directory where the zip is being extracted.
// * `ctx` - The packages context.
func extractDirectory(inputFile *zip.File, outputLocation string, ctx packages.Context) error {
	path := filepath.Join(outputLocation, inputFile.Name)

	err := os.MkdirAll(path, ctx.DirPerms)
	if err != nil {
		return fmt.Errorf("could not extract directory from package: %v", err)
	}

	return nil
}

// Extracts a file from a zip file and writes it to the system.
//
// You may want to use `extract()` instead, which extracts a full zip file.
//
// Note that `outputLocation` is not the final location of this file. The final location will  be
// `filepath.Join(outputLocation, inputFile.Name)`.
//
// Arguments:
// * `inputFile` - The file to extract.
// * `outputLocation` - The directory where the zip is being extracted.
// * `ctx` - The packages context.
func extractFile(inputFile *zip.File, outputLocation string, ctx packages.Context) error {
	path := filepath.Join(outputLocation, inputFile.Name)

	dir, _ := filepath.Split(path)
	err := os.MkdirAll(dir, ctx.DirPerms)
	if err != nil {
		return fmt.Errorf("could not create parent directories for symlink: %v", err)
	}

	reader, err := inputFile.Open()
	if err != nil {
		return fmt.Errorf("could not open symlink file in package: %v", err)
	}
	defer reader.Close()

	writer, err := os.Create(path)
	if err != nil {
		return fmt.Errorf("could not extract file from package: %v", err)
	}
	defer writer.Close()

	err = writer.Chmod(ctx.FilePerms)
	if err != nil {
		return fmt.Errorf("could not set permissions to extracted package file: %v", err)
	}

	_, err = io.Copy(writer, reader)
	if err != nil {
		return fmt.Errorf("could not write package file to system file: %v", err)
	}

	return nil
}

// Creates the symlinks to the package's exposed binaries.
//
// Arguments:
// * `manifest` - The package's manifest.
// * `ctx` - The packages context.
func exposeBinaries(manifest packages.Manifest, ctx packages.Context) error {
	for _, bin := range manifest.Exposes.Bin {
		if bin.Path == "" {
			bin.Path = bin.Name
		}

		symlinkLocation := filepath.Join(ctx.Path, "bin", bin.Name)
		originalLocation := filepath.Join(ctx.Path, "pkg", "packages", manifest.Package.Name, manifest.Package.Version, "bin", bin.Path)

		err := os.Symlink(originalLocation, symlinkLocation)
		if err != nil {
			return fmt.Errorf("could not expose `%v` binary through a symlink: %v", bin.Name, err)
		}
	}

	return nil
}
