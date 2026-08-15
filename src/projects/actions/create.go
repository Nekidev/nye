package actions

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/lithammer/dedent"
	"nyeki.dev/nye/projects"
)

// Initializes a package project with the specified name in the specified directory.
func CreatePackage(name string, path string) error {
	dirs := []string{
		path,
		filepath.Join(path, "src/bin"),
		filepath.Join(path, "dist"),
	}

	for _, path := range dirs {
		err := os.MkdirAll(path, 0o755)
		if err != nil {
			return fmt.Errorf("could not create directory/directories for project: %v", err)
		}
	}

	manifest := projects.Manifest{
		Package: projects.ManifestPackage{
			Name:    name,
			Version: "0.1.0",
		},
	}
	manifestPath := filepath.Join(path, "nye.toml")
	err := projects.SetManifest(manifestPath, manifest)
	if err != nil {
		return fmt.Errorf("could not set manifest: %v", err)
	}

	gitignore := `
		dist/
	`
	gitignore = dedent.Dedent(strings.TrimSpace(gitignore))
	err = os.WriteFile(filepath.Join(path, ".gitignore"), []byte(gitignore), 0o644)
	if err != nil {
		return fmt.Errorf("could not write .gitignore: %v", err)
	}

	return nil
}
