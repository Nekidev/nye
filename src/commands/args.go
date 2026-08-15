package commands

import (
	"fmt"
	"runtime/debug"

	"github.com/alecthomas/kong"
	"nyeki.dev/nye/commands/dev"
	"nyeki.dev/nye/packages"
)

type Args struct {
	System    bool             `short:"s" help:"Run the command in the system-wide installation directory (requires root)."`
	Version   bool             `short:"v" help:"Displays the current nye version."`
	Dev       dev.DevCommand   `cmd:"" aliases:"d" help:"Create and manage new packages."`
	Install   InstallCommand   `cmd:"" aliases:"i" help:"Install a package."`
	Uninstall UninstallCommand `cmd:"" aliases:"u" help:"Uninstall a package."`
}

func (args *Args) GetContext() (packages.Context, error) {
	return packages.GetContext(args.System)
}

func (args *Args) Run(ctx *kong.Context) error {
	if args.Version {
		info, ok := debug.ReadBuildInfo()
		if ok {
			fmt.Printf("nye %v - Nyeki's package manager.\n", info.Main.Version)
		} else {
			return fmt.Errorf("the current nye version could not be read. It's installed, though, and nothing points to that it won't work just fine.")
		}
	} else {
		ctx.PrintUsage(false)
	}

	return nil
}
