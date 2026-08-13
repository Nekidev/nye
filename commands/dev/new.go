package dev

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/fatih/color"
	"github.com/lithammer/dedent"
	"nyeki.dev/nye/packages"
)

type DevNewCommand struct {
	Name string `name:"name" help:"The name of the project."`
	Path string `name:"path" help:"The folder where the project will be created." default:"." type:"existingPath"`
}

func (cmd *DevNewCommand) Run() error {
	path, err := filepath.Abs(cmd.Path)
	if err != nil {
		return fmt.Errorf("could not resolve absolute path: %v", err)
	}

	cmd.Path = path

	if cmd.Name == "" {
		cmd.Name = filepath.Base(cmd.Path)
	}

	dirs := []string{
		cmd.Path,
		filepath.Join(cmd.Path, "src/bin"),
		filepath.Join(cmd.Path, "dist"),
	}

	for _, path := range dirs {
		err = os.MkdirAll(path, 0o755)
		if err != nil {
			return fmt.Errorf("could not create directory/directories for project: %v", err)
		}
	}

	manifest := packages.Manifest{
		Package: packages.ManifestPackage{
			Name:    cmd.Name,
			Version: "0.1.0",
		},
	}
	manifestPath := filepath.Join(cmd.Path, "Nye.toml")
	err = packages.SetManifest(manifestPath, manifest)
	if err != nil {
		return fmt.Errorf("could not set manifest: %v", err)
	}

	gitignore := `
		dist/
	`
	gitignore = dedent.Dedent(strings.TrimSpace(gitignore))
	err = os.WriteFile(filepath.Join(cmd.Path, ".gitignore"), []byte(gitignore), 0o644)
	if err != nil {
		return fmt.Errorf("could not write .gitignore: %v", err)
	}

	blue := color.New(color.FgBlue)
	fmt.Printf("Created project %v at %v.\n", blue.Sprint(cmd.Name), blue.Sprint(cmd.Path))

	return nil
}
