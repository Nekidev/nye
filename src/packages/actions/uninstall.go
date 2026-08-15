package actions

import (
	"fmt"
	"os"
	"path/filepath"

	"nyeki.dev/nye/packages"
	"nyeki.dev/nye/utils"
)

func UninstallPackage(ctx packages.Context, name string) ([]packages.Manifest, error) {
	manifests := []packages.Manifest{}

	packageDir := filepath.Join(ctx.Path, "pkg", "packages", name)
	exists, err := utils.Exists(packageDir)
	if err != nil {
		return nil, fmt.Errorf("could not check if package existed: %v", err)
	}
	if !exists {
		return nil, fmt.Errorf("there's no package `%v` installed. did you forget to use `--system`?", name)
	}

	entries, err := os.ReadDir(packageDir)
	if err != nil {
		return nil, fmt.Errorf("could not read the package's versions: %v", err)
	}

	for _, entry := range entries {
		packageVersionLocation := filepath.Join(packageDir, entry.Name())
		manifest, err := uninstallPackageVersion(ctx, packageVersionLocation)
		if err != nil {
			_, version := filepath.Split(entry.Name())
			return nil, fmt.Errorf("could not uninstall `%v %v`: %v", name, version, err)
		}
		manifests = append(manifests, manifest)
	}

	err = os.Remove(packageDir)
	if err != nil {
		return nil, fmt.Errorf("could not remove package's directory: %v", err)
	}

	return manifests, nil
}

func uninstallPackageVersion(ctx packages.Context, location string) (packages.Manifest, error) {
	manifestPath := filepath.Join(location, "package.toml")
	manifest, err := packages.GetManifestFromFile(manifestPath)
	if err != nil {
		return packages.Manifest{}, fmt.Errorf("could not read package manifest: %v", err)
	}

	err = os.RemoveAll(location)
	if err != nil {
		return packages.Manifest{}, fmt.Errorf("could not delete package files: %v", err)
	}

	err = uninstallPackageVersionExposedBins(ctx, manifest)
	if err != nil {
		return packages.Manifest{}, fmt.Errorf("could not remove exposed binary simlinks: %v", err)
	}

	return manifest, nil
}

func uninstallPackageVersionExposedBins(ctx packages.Context, manifest packages.Manifest) error {
	binsDir := filepath.Join(ctx.Path, "bin")

	for _, bin := range manifest.Exposes.Bin {
		linkPath := filepath.Join(binsDir, bin.Name)

		err := os.Remove(linkPath)
		if err != nil {
			return fmt.Errorf("could not remove symlink `%v`: %v", linkPath, err)
		}
	}

	return nil
}
