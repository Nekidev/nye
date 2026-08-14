package commands

import (
	"nyeki.dev/nye/commands/dev"
	"nyeki.dev/nye/packages"
)

type Args struct {
	System  bool           `short:"s" help:"Run the command in the system-wide installation directory."`
	Dev     dev.DevCommand `cmd:"" aliases:"d" help:"Create and manage new packages."`
	Install InstallCommand `cmd:"" aliases:"i" help:"Install a package."`
}

func (args *Args) GetContext() (packages.Context, error) {
	return packages.GetContext(args.System)
}
