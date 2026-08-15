package commands

import (
	"fmt"
	"os"
	"runtime/debug"

	"github.com/fatih/color"
	"nyeki.dev/nye/commands/dev"
	"nyeki.dev/nye/packages"
	"nyeki.dev/nye/utils"
)

var PackageDescription = "Nyeki's package manager."

type Args struct {
	System    bool             `short:"s" help:"Run the command in the system-wide installation directory (requires root)."`
	Version   VersionFlag      `short:"v" help:"Displays the current nye version."`
	Target    TargetFlag       `short:"t" help:"Displays your system's target."`
	Install   InstallCommand   `cmd:"" aliases:"i" help:"Install a package."`
	Uninstall UninstallCommand `cmd:"" aliases:"u" help:"Uninstall a package."`
	Dev       dev.DevCommand   `cmd:"" aliases:"d" help:"Create and manage new packages."`
}

type VersionFlag bool

func (f *VersionFlag) BeforeReset() error {
	info, ok := debug.ReadBuildInfo()
	if ok {
		fmt.Printf("nye %v - %v\n", info.Main.Version, PackageDescription)
		os.Exit(0)
	} else {
		fmt.Println("the current nye version could not be read. It's installed, though, and nothing points to that it won't work just fine.")
		os.Exit(1)
	}
	return nil
}

type TargetFlag bool

func (f *TargetFlag) BeforeReset() error {
	target := utils.GetCurrentTarget()
	blue := color.New(color.FgBlue)

	fmt.Printf("Your system's target is %v.", blue.Sprint(target))

	if !utils.IsSupportedTarget(target) {
		red := color.New(color.FgRed)
		red.Print(" Your system's target is not supported by nye. Use at your own risk.")
	}

	fmt.Println()

	os.Exit(0)
	return nil
}

func (args *Args) GetContext() (packages.Context, error) {
	return packages.GetContext(args.System)
}
