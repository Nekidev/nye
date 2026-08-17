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
		return packages.Manifest{}, fmt.Errorf("could not remove exposed binary wrappers: %v", err)
	}

	err = uninstallPackageVersionExposedEtcs(ctx, manifest)
	if err != nil {
		return packages.Manifest{}, fmt.Errorf("could not remove exposed etcs: %v", err)
	}

	err = uninstallPackageVersionExposedEnvs(ctx, manifest)
	if err != nil {
		return packages.Manifest{}, fmt.Errorf("could not uninstall package's env variables: %v", err)
	}

	return manifest, nil
}

func uninstallPackageVersionExposedBins(ctx packages.Context, manifest packages.Manifest) error {
	binsDir := filepath.Join(ctx.Path, "bin")

	for _, bin := range manifest.Exposes.Bin {
		wrapperPath := filepath.Join(binsDir, bin.Name)

		err := os.Remove(wrapperPath)
		if err != nil {
			return fmt.Errorf("could not remove binary wrapper `%v`: %v", wrapperPath, err)
		}
	}

	return nil
}

func uninstallPackageVersionExposedEtcs(ctx packages.Context, manifest packages.Manifest) error {
	etcsSymlink := filepath.Join(ctx.Path, "etc", manifest.Package.Name)

	err := os.Remove(etcsSymlink)
	if err != nil {
		return fmt.Errorf("could not remove etcs symlink at `%v`: %v", etcsSymlink, err)
	}

	return nil
}

func uninstallPackageVersionExposedEnvs(ctx packages.Context, manifest packages.Manifest) error {
	variablesDir := filepath.Join(ctx.Path, "pkg", "env")

	for _, env := range manifest.Exposes.Env {
		variableDir := filepath.Join(variablesDir, env.Name)
		variablePackageDir := filepath.Join(variableDir, manifest.Package.Name)
		variablePackageVersionLink := filepath.Join(variablePackageDir, manifest.Package.Version)

		err := os.Remove(variablePackageVersionLink)
		if err != nil {
			return fmt.Errorf("could not remove env var file symlink at `%v`: %v", variablePackageVersionLink, err)
		}

		empty, err := utils.IsEmpty(variablePackageDir)
		if err != nil {
			return fmt.Errorf("could not check if directory at `%v` was empty: %v", variablePackageDir, err)
		}
		if empty {
			err := os.Remove(variablePackageDir)
			if err != nil {
				return fmt.Errorf("could not remove env var package dir at `%v`: %v", variablePackageDir, err)
			}

			empty, err = utils.IsEmpty(variableDir)
			if err != nil {
				return fmt.Errorf("could not check if directory at `%v` was empty: %v", variableDir, err)
			}
			if empty {
				err := os.Remove(variableDir)
				if err != nil {
					return fmt.Errorf("could not remove env var dir at `%v`: %v", variableDir, err)
				}
			}
		}
	}

	return nil
}
