package commands

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/fatih/color"
	"nyeki.dev/nye/packages"
	pactions "nyeki.dev/nye/packages/actions"
	"nyeki.dev/nye/registries"
	ractions "nyeki.dev/nye/registries/actions"
)

type InstallCommand struct {
	Names []string "arg:\"\" help:\"The names of the packages to download and install. They can be prefixed by `registry-name/` to specify a registry, otherwise the default registry will be used.\""
	Paths []string `flag:"path" short:"p" help:"The path to the package's zip file." type:"existingPath"`
}

func (cmd *InstallCommand) Run(ctx packages.Context) error {
	// Path
	packageZipPaths := []string{}

	if len(cmd.Names) > 0 {
		config, err := registries.GetConfig()
		if err != nil {
			return fmt.Errorf("could not read registries configuration: %v", err)
		}

		for _, name := range cmd.Names {
			registryName := config.Default

			if strings.Contains(name, "/") {
				parts := strings.SplitN(name, "/", 2)
				registryName = parts[0]
				name = parts[1]
			}

			registry := config.GetRegistry(registryName)
			if registry == nil {
				return fmt.Errorf("there is no registry called `%v` in `registries.toml`", registryName)
			}

			dirPath := filepath.Join(ctx.Path, "tmp")
			err = os.MkdirAll(dirPath, 0o700)
			if err != nil {
				return fmt.Errorf("could not create temporary directory for downloads at `%v`: %v", dirPath, err)
			}

			file, err := os.CreateTemp(dirPath, "*.zip")
			if err != nil {
				return fmt.Errorf("could not create temporary file for download: %v", err)
			}
			defer file.Close()
			defer os.Remove(file.Name())

			err = ractions.DownloadPackage(registry.Url, name, file)
			if err != nil {
				return fmt.Errorf("could not download package file: %v", err)
			}

			packageZipPaths = append(packageZipPaths, file.Name())
		}
	}

	packageZipPaths = append(packageZipPaths, cmd.Paths...)

	// Package name to installed version
	installed := map[string]string{}

	for _, filePath := range packageZipPaths {
		manifest, err := pactions.InstallPackage(ctx, filePath)
		if err != nil {
			return fmt.Errorf("could not install package: %v", err)
		}

		installed[manifest.Package.Name] = manifest.Package.Version
	}

	fmt.Println("Done! The following packages have been installed:")
	blue := color.New(color.FgBlue)

	var index = 1
	for packageName, packageVersion := range installed {
		fmt.Printf("%v. %v\n", index, blue.Sprintf("%v v%v", packageName, packageVersion))
		index += 1
	}

	return nil
}
