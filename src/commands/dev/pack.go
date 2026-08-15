package dev

import (
	"fmt"

	"github.com/fatih/color"
	"nyeki.dev/nye/projects"
	"nyeki.dev/nye/projects/actions"
)

type DevPackCommand struct{}

func (cmd *DevPackCommand) Run() error {
	ctx, err := projects.GetContextCwd()
	if err != nil {
		return fmt.Errorf("could not get context for working directory's package: %v", err)
	}

	zip, err := actions.PackPackage(ctx.Path)

	blue := color.New(color.FgBlue)
	fmt.Printf("Done! Your packed package is in %v.\n", blue.Sprint(zip))

	return nil
}
