package dev

type DevCommand struct {
	New DevNewCommand `cmd:"" help:"Create a new package project."`
	Pack DevPackCommand `cmd:"" help:"Pack the current project into an installable package."`
}
