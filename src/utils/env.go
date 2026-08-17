package utils

import "os"

var (
	EnvNyeInstallationEtc = os.Getenv("NYE_INSTALLATION_ETC")
	EnvNyeInstallationBin = os.Getenv("NYE_INSTALLATION_BIN")
)
