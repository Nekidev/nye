FROM mcr.microsoft.com/devcontainers/rust:2-1-trixie

RUN sed -i 's|http://deb.debian.org|https://deb.debian.org|g' \
   /etc/apt/sources.list.d/debian.sources

# Include lld linker to improve build times either by using environment variable
# RUSTFLAGS="-C link-arg=-fuse-ld=lld" or with Cargo's configuration file (i.e see .cargo/config.toml).
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
   && apt-get -y install clang lld \
   && apt-get autoremove -y && apt-get clean -y

RUN apt install -y python3 pipx

RUN pipx install djlint

RUN rustup component add rustfmt --toolchain nightly
