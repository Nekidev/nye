package commands

import (
	"fmt"

	"github.com/fatih/color"
	"nyeki.dev/nye/packages"
	"nyeki.dev/nye/packages/actions"
)

type InstallCommand struct {
	Path string `help:"The path to the package's zip file." type:"existingPath"`
}

func (cmd *InstallCommand) Run(ctx packages.Context) error {
	manifest, err := actions.InstallPackage(ctx, cmd.Path)
	if err != nil {
		return fmt.Errorf("could not install package: %v", err)
	}

	blue := color.New(color.FgBlue)
	fmt.Printf("Done! %v is now installed.\n", blue.Sprintf("%v v%v", manifest.Package.Name, manifest.Package.Version))

	return nil
}
