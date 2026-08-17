package packages

import (
	"archive/zip"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"slices"

	"github.com/pelletier/go-toml/v2"
	"nyeki.dev/nye/projects"
	"nyeki.dev/nye/utils"
)

// A nye.toml manifest.
type Manifest struct {
	Package  ManifestPackage  `toml:"package"`
	Exposes  ManifestExposes  `toml:"exposes"`
	Consumes ManifestConsumes `toml:"consumes"`
}

type ManifestPackage struct {
	Name    string `toml:"name" validate:"required,kebab-case,min=1,max=32"`
	Version string `toml:"version" validate:"required,semver,max=32"`
	Target  string `toml:"target" validate:"required,supported-target"`
}

type ManifestExposes struct {
	Bin []ManifestExposesBin `toml:"bin" validate:"unique=Name"` // Exposed symlinks on installation to the package's bundled binaries
	Env []ManifestExposesEnv `toml:"env" validate:"unique=Name"` // Exposed environment variables. They're not set to the system's env vars.
}

type ManifestExposesBin struct {
	Name string `toml:"name" validate:"required,min=1,max=32,safe-path-segment"`
	Path string `toml:"path" validate:"required,min=1,safe-path"`
}

type ManifestExposesEnv struct {
	Name  string `toml:"name" validate:"required,min=1,max=32,env-var-name"`
	Value string `toml:"value" validate:"required,max=1024"`
}

type ManifestConsumes struct {
	Env []ManifestConsumesEnv `toml:"name" validate:"unique=Name"`
}

type ManifestConsumesEnv struct {
	Name      string `toml:"name" validate:"required,min=1,max=32,env-var-name"`
	Separator string `toml:"separator" validate:"required,max=32"`
}

// Reads a manifest from a file in the system.
//
// Unlike `GetManifestFromZip()`, this function does not validate the manifest on system data.
// The validation is done on syntax and individual values only. This is due this function being
// intended to be used on already-installed packages, whose manifests have already been validated
// by `GetManifestFromZip()`.
//
// Arguments:
// * `path` - The path to the manifest file.
//
// Returns:
// * `Manifest` - The manifest, if no error occurred.
// * `error` - An error if occurred, `nil` otherwise.
func GetManifestFromFile(path string) (Manifest, error) {
	contents, err := os.ReadFile(path)
	if err != nil {
		return Manifest{}, fmt.Errorf("could not read manifest file: %v", err)
	}

	manifest := Manifest{}
	err = toml.Unmarshal(contents, &manifest)
	if err != nil {
		return Manifest{}, fmt.Errorf("could not parse manifest file: %v", err)
	}

	err = utils.ValidateStruct(&manifest)
	if err != nil {
		return Manifest{}, fmt.Errorf("manifest was not valid: %v", err)
	}

	return manifest, nil
}

func GetManifestFromZip(path string) (Manifest, error) {
	zipper, err := zip.OpenReader(path)
	if err != nil {
		return Manifest{}, fmt.Errorf("could not read package file: %v", err)
	}
	defer zipper.Close()

	for _, file := range zipper.File {
		info := file.FileInfo()
		if info.Name() == "package.toml" {
			file, err := file.Open()
			if err != nil {
				return Manifest{}, fmt.Errorf("could not open manifestin package file: %v", err)
			}
			defer file.Close()

			contents, err := io.ReadAll(file)
			if err != nil {
				return Manifest{}, fmt.Errorf("could not read manifest in package file: %v", err)
			}

			manifest := Manifest{}
			err = toml.Unmarshal(contents, &manifest)
			if err != nil {
				return Manifest{}, fmt.Errorf("could not parse manifest in package file: %v", err)
			}

			err = validateManifest(manifest, zipper)
			if err != nil {
				return Manifest{}, fmt.Errorf("manifest in package file was invalid: %v", err)
			}

			return manifest, nil
		}
	}

	return Manifest{}, fmt.Errorf("the package file did not contain a manifest")
}

func validateManifest(manifest Manifest, zipper *zip.ReadCloser) error {
	if err := utils.ValidateStruct(manifest); err != nil {
		return err
	}

	if err := validateManifestExposedBins(manifest, zipper); err != nil {
		return err
	}

	return nil
}

func validateManifestExposedBins(manifest Manifest, zipper *zip.ReadCloser) error {
	exposedNames := []string{}

	for _, bin := range manifest.Exposes.Bin {
		if slices.Contains(exposedNames, bin.Name) {
			return fmt.Errorf("there are two or more binaries exposed under the same name, `%v`, which is not allowed", bin.Name)
		}
		exposedNames = append(exposedNames, bin.Name)

		if bin.Path == "" {
			bin.Path = bin.Name
		}

		found := false

		for _, file := range zipper.File {
			info := file.FileInfo()

			if info.IsDir() {
				continue
			}

			if file.Name == filepath.Join("bin", bin.Path) {
				found = true
				break
			}
		}

		if !found {
			return fmt.Errorf("exposed binary `%v` at `%v` was not in package", bin.Name, bin.Path)
		}
	}

	return nil
}

func FromProjectManifest(manifest projects.Manifest, target string) Manifest {
	exposedBins := []ManifestExposesBin{}
	exposedEnvs := []ManifestExposesEnv{}
	consumedEnvs := []ManifestConsumesEnv{}

	exposedForTarget := manifest.Exposes.ForTarget(target)

	for _, bin := range exposedForTarget.Bin {
		if bin.Path == "" {
			bin.Path = bin.Name
		}

		exposedBins = append(exposedBins, ManifestExposesBin{
			Name: bin.Name,
			Path: bin.Path,
		})
	}

	for _, env := range exposedForTarget.Env {
		exposedEnvs = append(exposedEnvs, ManifestExposesEnv{
			Name:  env.Name,
			Value: env.Value,
		})
	}

	for _, env := range manifest.Consumes.Env {
		consumedEnvs = append(consumedEnvs, ManifestConsumesEnv{
			Name:      env.Name,
			Separator: env.Separator,
		})
	}

	result := Manifest{
		Package: ManifestPackage{
			Name:    manifest.Package.Name,
			Version: manifest.Package.Version,
			Target:  target,
		},
		Exposes: ManifestExposes{
			Bin: exposedBins,
			Env: exposedEnvs,
		},
		Consumes: ManifestConsumes{
			Env: consumedEnvs,
		},
	}

	return result
}
