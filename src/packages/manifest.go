package packages

import (
	"archive/zip"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"slices"

	"github.com/pelletier/go-toml/v2"
	"nyeki.dev/nye/utils"
)

// A nye.toml manifest.
type Manifest struct {
	Package ManifestPackage `toml:"package"`
	Exposes ManifestExposes `toml:"exposes"`
}

type ManifestPackage struct {
	Name    string `toml:"name" validate:"required,kebabCase,min=1,max=32"`
	Version string `toml:"version" validate:"required,semver,max=32"`
}

type ManifestExposes struct {
	// Exposed symlinks on installation to the package's bundled binaries
	Bin []ManifestExposesBinary `toml:"bin"`
}

type ManifestExposesBinary struct {
	Name string `toml:"name" validate:"required,min=1,max=32,safePathSegment"`
	Path string `toml:"path" validate:"required,min=1,safePath"`
}

// Reads a manifest from a file in the system.
//
// Unlike `GetManifestFromZip()`, this function does not validate the manifest.
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
