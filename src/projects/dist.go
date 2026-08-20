package projects

import (
	"fmt"
	"path/filepath"
)

func GetPackDistPath(ctx Context, target string) string {
	return filepath.Join(ctx.Path, "dist", fmt.Sprintf("nye-%v-v%v-for-%v-pack.zip", ctx.Manifest.Package.Name, ctx.Manifest.Package.Version, target))
}
