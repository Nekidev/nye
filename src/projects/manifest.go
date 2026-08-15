package projects

import (
	"fmt"
	"os"
	"path/filepath"
	"slices"

	"github.com/pelletier/go-toml/v2"
	"nyeki.dev/nye/utils"
)

// A nye.toml manifest.
type Manifest struct {
	Package ManifestPackage `toml:"package" validate:"required"`
	Targets ManifestTargets `toml:"targets" validate:"required,unique,dive,keys,supported-target,endkeys"`
	Exposes ManifestExposes `toml:"exposes"`
}

type ManifestPackage struct {
	Name    string `toml:"name" validate:"required,min=1,max=32,kebab-case"`
	Version string `toml:"version" validate:"required,semver,max=32"`
}

type ManifestTargets map[string]ManifestTarget

type ManifestTarget struct {
	Source string `toml:"source" validate:"safe-path"`
}

type ManifestExposes struct {
	// Exposed symlinks on installation to the package's bundled binaries
	Bin []ManifestExposesBinary `toml:"bin" validate:"unique=Name"`
}

func (exposes *ManifestExposes) ForTarget(target string) ManifestExposes {
	bins := []ManifestExposesBinary{}

	for _, bin := range exposes.Bin {
		if len(bin.Targets) > 0 {
			if slices.Contains(bin.Targets, target) {
				bins = append(bins, bin)
			}
		} else {
			bins = append(bins, bin)
		}
	}

	return ManifestExposes{Bin: bins}
}

type ManifestExposesBinary struct {
	Name string `toml:"name" validate:"required,min=1,max=32,safe-path-segment"`
	Path string `toml:"path" validate:"safe-path"`
	Targets []string `toml:"targets" validate:"unique,dive,supported-target"`
}

func GetManifest(path string) (Manifest, error) {
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

	err = validateExposedBins(path, manifest)
	if err != nil {
		return Manifest{}, fmt.Errorf("failed to validate exposed binaries: %v", err)
	}

	return manifest, nil
}

func SetManifest(path string, content Manifest) error {
	bytes, err := toml.Marshal(content)
	if err != nil {
		return fmt.Errorf("could not marshal manifest to TOML: %v", err)
	}

	err = os.WriteFile(path, bytes, 0o644)
	if err != nil {
		return fmt.Errorf("could not write manifest to path %v: %v", path, err)
	}

	return nil
}

func validateExposedBins(path string, manifest Manifest) error {
	dir, _ := filepath.Split(path)

	exposedNames := []string{}

	for _, bin := range manifest.Exposes.Bin {
		if slices.Contains(exposedNames, bin.Name) {
			return fmt.Errorf("there are two or more exposed binaries with the name `%v`, only one binary can be exposed per name", bin.Name)
		}
		exposedNames = append(exposedNames, bin.Name)

		if bin.Path == "" {
			bin.Path = bin.Name
		}

		binPath := filepath.Join(dir, "src", "bin", bin.Path)
		stat, err := os.Stat(binPath)
		if err != nil {
			return fmt.Errorf("could not validate exposed binary `%v`: %v", binPath, err)
		} else {
			if stat.IsDir() {
				return fmt.Errorf("the directory `%v` cannot be exposed as a binary", binPath)
			}
		}
	}

	return nil
}
