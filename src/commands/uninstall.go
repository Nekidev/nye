package commands

import (
	"fmt"

	"github.com/fatih/color"
	"nyeki.dev/nye/packages"
	"nyeki.dev/nye/packages/actions"
)

type UninstallCommand struct {
	Name string `arg:"" help:"The name of the package to uninstall."`
}

func (cmd *UninstallCommand) Run(ctx packages.Context) error {
	manifests, err := actions.UninstallPackage(ctx, cmd.Name)
	if err != nil {
		return fmt.Errorf("could not uninstall package: %v", err)
	}

	fmt.Println("Done! The following packages have been uninstalled:")
	blue := color.New(color.FgBlue)

	for i, manifest := range manifests {
		fmt.Printf("%v. %v\n", i+1, blue.Sprintf("%v v%v", manifest.Package.Name, manifest.Package.Version))
	}

	return nil
}
