package actions

import (
	"archive/zip"
	"fmt"
	"io"
	"os"
	"path/filepath"

	"nyeki.dev/nye/projects"
)

// Packs a package project.
//
// Arguments:
// * `path` - The directory where the `nye.toml` manifest is located.
//
// Returns:
// * `string` - The path to the packaged pack relative to the current working directory.
// * `error` - If the package failed to be packed.
func PackPackage(path string) (string, error) {
	ctx, err := projects.GetContext(path)
	if err != nil {
		return "", fmt.Errorf("could not get context to pack: %v", err)
	}

	distDir := filepath.Join(ctx.Path, "dist")
	err = os.MkdirAll(distDir, 0o755)
	if err != nil {
		return "", fmt.Errorf("could not create dist dir: %v", err)
	}

	zipName := filepath.Join(distDir, fmt.Sprintf("nye-%v-v%v-pack.zip", ctx.Manifest.Package.Name, ctx.Manifest.Package.Version))
	zipFile, err := os.Create(zipName)
	if err != nil {
		return "", fmt.Errorf("could not create pack file: %v", err)
	}
	defer zipFile.Close()

	zipWriter := zip.NewWriter(zipFile)
	defer zipWriter.Close()

	err = putFileInZip(zipWriter, filepath.Join(ctx.Path, "nye.toml"), "package.toml")
	if err != nil {
		return "", fmt.Errorf("could not put manifest in zip file: %v", err)
	}

	srcDir := filepath.Join(ctx.Path, "src")
	err = filepath.WalkDir(srcDir, func(path string, dir os.DirEntry, err error) error {
		if err != nil {
			return err
		}

		info, err := os.Stat(path)
		if err != nil {
			return fmt.Errorf("could not get information about path in src directory: %v", err)
		}

		if info.IsDir() {
			return nil
		}

		rel, err := filepath.Rel(srcDir, path)
		if err != nil {
			return fmt.Errorf("could not get relative path for file to pack: %v", err)
		}

		err = putFileInZip(zipWriter, path, rel)
		if err != nil {
			return fmt.Errorf("could not put file to pack in zip: %v", err)
		}

		return nil
	})
	if err != nil {
		return "", fmt.Errorf("could not pack project's contents into zip: %v", err)
	}

	cwd, err := os.Getwd()
	if err != nil {
		return "", fmt.Errorf("could not get CWD (pack was successfully created nonetheless): %v", err)
	}

	zipRel, err := filepath.Rel(cwd, zipName)
	if err != nil {
		return "", fmt.Errorf("could not get zip path relative to CWD (pack was successfully created nonetheless): %v", err)
	}

	return zipRel, nil
}

// Puts a file into a zip file.
func putFileInZip(file *zip.Writer, input string, output string) error {
	inputFile, err := os.Open(input)
	if err != nil {
		return fmt.Errorf("could not read zip's input file: %v", err)
	}
	defer inputFile.Close()

	outputEntry, err := file.Create(output)
	if err != nil {
		return fmt.Errorf("could not create entry in zip file for output path: %v", err)
	}

	_, err = io.Copy(outputEntry, inputFile)
	if err != nil {
		return fmt.Errorf("could not insert input file into zip file: %v", err)
	}

	return nil
}
