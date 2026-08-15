package packages

import (
	"fmt"
	"os"
	"os/user"
)

type Context struct {
	Path      string      // The path to the namespace directory where to work. This will be `/` for `--system` executions, `/usr/{username}` otherwise.
	DirPerms  os.FileMode // Permissions to create directories with.
	FilePerms os.FileMode // Permissions to create files with.
}

func GetContext(isSystem bool) (Context, error) {
	isRoot := os.Geteuid() == 0

	if isSystem && !isRoot {
		return Context{}, fmt.Errorf("cannot run command as system without being root")
	}

	user, err := user.Current()
	if err != nil {
		return Context{}, fmt.Errorf("could not get current user")
	}

	systemPath := "/"
	userPath := fmt.Sprintf("/usr/%v", user.Username)

	if isSystem {
		return Context{Path: systemPath, DirPerms: 0o755, FilePerms: 0o744}, nil
	} else {
		return Context{Path: userPath, DirPerms: 0o700, FilePerms: 0o700}, nil
	}
}
