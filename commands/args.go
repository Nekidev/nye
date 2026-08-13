package commands

import (
	"nyeki.dev/nye/commands/dev"
)

type Args struct {
	Dev dev.DevCommand `cmd:"" help:"Create and manage new packages."`
}
