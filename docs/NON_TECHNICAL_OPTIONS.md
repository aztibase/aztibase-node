# Non-Technical Friendly Options

Current validator setup requires SSH, Linux CLI, and compiling from source. These options would lower the barrier for non-technical participants.

## Option 1: Pre-Built Binaries

Publish release binaries on GitHub Releases instead of requiring friends to compile from source.

- Targets: `aztibase-linux-x86_64`, `aztibase-linux-aarch64`
- Setup script downloads a tarball instead of cloning + `cargo build`
- Eliminates: Rust toolchain install, 5-15 min compile, build failures on low-RAM VPS
- Effort: Low-medium (CI pipeline + release workflow)

## Option 2: Docker Image

`docker run aztibase/validator` with environment variables for configuration.

- No compile, no system deps, one command
- Config via env vars: `NETWORK`, `BOOT_NODES`, `GENESIS_URL`, `NODE_NAME`
- Volume mount for persistent data and validator key
- Effort: Low (Dockerfile + entrypoint script)

## Option 3: Web-Based Genesis Ceremony

Simple web page replacing the CLI genesis workflow.

- Coordinator creates a ceremony link
- Friends paste their public keys (address, Ed25519 pubkey, BLS pubkey) into a form
- Coordinator reviews, downloads `genesis.toml`
- No CLI, no hex string copy-paste errors
- Effort: Medium (static site or simple backend)

## Option 4: One-Click Cloud Deploy

"Deploy to DigitalOcean" style button that provisions infrastructure automatically.

- Terraform/Pulumi template for DigitalOcean, Hetzner, or AWS Lightsail
- User clicks a link, enters API key + genesis file, gets a running validator
- Could also be a DigitalOcean Marketplace image or Docker droplet
- Effort: Medium-high (cloud templates + testing across providers)

## Priority Recommendation

Pre-built binaries (Option 1) + Docker (Option 2) give the biggest UX improvement for the least effort. Options 3 and 4 are nice-to-haves for broader adoption.
