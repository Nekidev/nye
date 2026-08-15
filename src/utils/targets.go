package utils

import (
	"fmt"
	"runtime"
)

var SupportedTargets = []string{
	"linux-386",
	"linux-amd64",
	"linux-amd64p32",
	"linux-arm",
	"linux-arm64",
	"linux-arm64be",
	"linux-armbe",
	"linux-loong64",
	"linux-mips",
	"linux-mips64",
	"linux-mips64le",
	"linux-mips64p32",
	"linux-mips64p32le",
	"linux-mipsle",
	"linux-ppc",
	"linux-ppc64",
	"linux-ppc64le",
	"linux-riscv",
	"linux-riscv64",
	"linux-s390",
	"linux-s390x",
	"linux-sparc",
	"linux-sparc64",
	"linux-wasm",
}

// Returns the target of the current system.
//
// The target may not be supported by the package manager. To check so, call
// `IsSupportedTarget()`.
func GetCurrentTarget() string {
	return fmt.Sprintf("%v-%v", runtime.GOOS, runtime.GOARCH)
}
