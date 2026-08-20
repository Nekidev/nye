package dev

type DevCommand struct {
	New     DevNewCommand     `cmd:"" help:"Create a new package project."`
	Pack    DevPackCommand    `cmd:"" help:"Pack the current project into an installable package."`
	Publish DevPublishCommand `cmd:"" help:"Publish the current project to a registry."`
}
