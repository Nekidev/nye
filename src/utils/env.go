package utils

import "os"

var (
	EnvNyeInstallationEtc = os.Getenv("NYE_INSTALLATION_ETC")
	EnvNyeInstallationBin = os.Getenv("NYE_INSTALLATION_BIN")
	EnvNyeInstallationEnv = os.Getenv("NYE_INSTALLATION_ENV")
)
