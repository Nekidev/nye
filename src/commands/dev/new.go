package dev

import (
	"fmt"
	"path/filepath"

	"github.com/fatih/color"
	"nyeki.dev/nye/projects/actions"
)

type DevNewCommand struct {
	Name string `help:"The name of the project."`
	Path string `help:"The folder where the project will be created." default:"." type:"existingPath"`
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

	err = actions.CreatePackage(cmd.Name, cmd.Path)
	if err != nil {
		return fmt.Errorf("could not create package: %v", err)
	}

	blue := color.New(color.FgBlue)
	fmt.Printf("Created project %v at %v.\n", blue.Sprint(cmd.Name), blue.Sprint(cmd.Path))

	return nil
}
