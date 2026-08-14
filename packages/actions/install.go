package actions

import (
	"archive/zip"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"

	"github.com/pelletier/go-toml/v2"
	"nyeki.dev/nye/packages"
)

// Installs a package.
//
// Arguments:
// * `ctx` - The current execution context.
// * `path` - The path to the package's zip file.
func InstallPackage(ctx packages.Context, path string) (packages.Manifest, error) {
	zipReader, err := zip.OpenReader(path)
	if err != nil {
		return packages.Manifest{}, fmt.Errorf("could not read package file: %v", err)
	}
	defer zipReader.Close()

	if err = os.MkdirAll(filepath.Join(ctx.Path, "pkg", "tmp"), ctx.DirPerms); err != nil {
		return packages.Manifest{}, fmt.Errorf("could not create temporary working dir: %v", err)
	}

	manifest, err := getManifest(zipReader)
	if err != nil {
		return packages.Manifest{}, fmt.Errorf("could not read manifest from package: %v", err)
	}

	packageDir := filepath.Join(ctx.Path, "pkg", "packages", manifest.Package.Name, manifest.Package.Version)
	if err = os.MkdirAll(packageDir, ctx.DirPerms); err != nil {
		return packages.Manifest{}, fmt.Errorf("could not create directories for package version")
	}

	if err := unzip(ctx, zipReader, packageDir); err != nil {
		return packages.Manifest{}, fmt.Errorf("could not extract package: %v", err)
	}

	err = createSymlinks(ctx, packageDir, manifest)
	if err != nil {
		return packages.Manifest{}, fmt.Errorf("could not create symlinks for exposed binaries: %v", err)
	}

	return manifest, nil
}

func getManifest(pack *zip.ReadCloser) (packages.Manifest, error) {
	for _, file := range pack.File {
		if file.Name == "package.toml" {
			file, err := file.Open()
			if err != nil {
				return packages.Manifest{}, fmt.Errorf("could not read package's manifest: %v", err)
			}

			contents, err := io.ReadAll(file)
			if err != nil {
				return packages.Manifest{}, fmt.Errorf("could not read package's manifest: %v", err)
			}

			manifest := packages.Manifest{}
			if err = toml.Unmarshal(contents, &manifest); err != nil {
				return manifest, fmt.Errorf("could not parse package's manifest: %v", err)
			}

			return manifest, nil
		}
	}

	return packages.Manifest{}, fmt.Errorf("the package zip did not have a manifest")
}

func unzip(ctx packages.Context, reader *zip.ReadCloser, destination string) error {
	for _, file := range reader.File {
		path := filepath.Join(destination, file.Name)

		// Security check: Prevent Zip Slip (path traversal)
		if !strings.HasPrefix(path, filepath.Clean(destination)+string(os.PathSeparator)) {
			continue
		}

		if file.FileInfo().IsDir() {
			if err := os.MkdirAll(path, ctx.DirPerms); err != nil {
				return fmt.Errorf("could not extract directory: %v", err)
			}
		} else {
			dir, _ := filepath.Split(path)
			if err := os.MkdirAll(dir, ctx.DirPerms); err != nil {
				return fmt.Errorf("could not create required directories: %v", err)
			}

			outputWriter, err := os.Create(path)
			if err != nil {
				return fmt.Errorf("could not extract file: %v", err)
			}

			if err = outputWriter.Chmod(ctx.FilePerms); err != nil {
				return fmt.Errorf("could not set permissions to extracted file: %v", err)
			}

			inputReader, err := file.Open()
			if err != nil {
				return fmt.Errorf("could not open file in zip: %v", err)
			}

			if _, err = io.Copy(outputWriter, inputReader); err != nil {
				return fmt.Errorf("could not extract file in zip to created file: %v", err)
			}
		}
	}

	return nil
}

// Creates the necessary symlinks to expose the package's exposed binaries to the user.
//
// Arguments:
// * `ctx` - The execution context.
// * `packageDir` - The directory where the installed package (including version) is installed. E.g. `/pkg/packages/busybox/1.0.0`.
// * `packageManifest` - The package's manifest, which contains the exposed binaries metadata.
func createSymlinks(ctx packages.Context, packageDir string, packageManifest packages.Manifest) error {
	for _, exposedBin := range packageManifest.Exposes.Bin {
		if exposedBin.Name == "" && exposedBin.Path == "" {
			continue
		}

		if exposedBin.Path == "" {
			exposedBin.Path = exposedBin.Name
		}

		if exposedBin.Name == "" {
			_, file := filepath.Split(exposedBin.Path)
			exposedBin.Name = file
		}

		namespaceBin := filepath.Join(ctx.Path, "bin")
		if err := os.MkdirAll(namespaceBin, ctx.DirPerms); err != nil {
			return fmt.Errorf("could not create namespace's bin directory: %v", err)
		}

		original := filepath.Join(packageDir, "bin", exposedBin.Path)
		symlink := filepath.Join(ctx.Path, "bin", exposedBin.Name)

		err := os.Symlink(original, symlink)
		if err != nil {
			return fmt.Errorf("could not create symlink for exposed binary %v: %v", exposedBin.Name, err)
		}
	}

	return nil
}
