# Ethrex Replay Web

A Phoenix/Elixir web application for executing and proving Ethereum blocks using [ethrex-replay](https://github.com/lambdaclass/ethrex-replay). Generate zero-knowledge proofs of block execution with a beautiful, real-time interface.

![Ethrex Replay Web](https://img.shields.io/badge/Elixir-1.15+-4B275F?style=flat-square&logo=elixir)
![Phoenix](https://img.shields.io/badge/Phoenix-1.8+-F05423?style=flat-square&logo=phoenixframework)
![License](https://img.shields.io/badge/License-MIT-blue?style=flat-square)

## Features

- **Job Configuration** - Configure all ethrex-replay options through an intuitive UI
  - 8 ZKVMs: SP1, RISC0, OpenVM, ZisK, Jolt, Nexus, Pico, Ziren
  - Actions: Execute or Prove
  - Resources: CPU or GPU
  - Networks: Mainnet, Sepolia, Hoodi, Holesky
  - Block selection with latest block support

- **Real-time Execution** - Watch proof generation in real-time
  - Live log streaming via WebSockets
  - Auto-scrolling log viewer
  - Syntax-highlighted output

- **Job Management** - Queue and track multiple jobs
  - One job at a time with automatic queue processing
  - Job history with SQLite persistence
  - Cancel running jobs

- **System Information** - Monitor your hardware
  - CPU, GPU, and RAM detection
  - ZKVM compatibility matrix
  - GPU availability status

## Screenshots

The application features a dark mode interface inspired by [ethproofs.org](https://ethproofs.org):

- **Dashboard** - Job submission form with command preview
- **Job View** - Real-time logs and execution results
- **History** - Filterable job history table
- **System** - Hardware specifications and compatibility info

## Quick Start

### Prerequisites

- [Elixir](https://elixir-lang.org/install.html) 1.15+
- [Rust](https://rustup.rs/) (for running ethrex-replay)
- SQLite3

### Installation

```bash
# Clone the repository (if not already done)
git clone https://github.com/lambdaclass/ethrex-replay.git
cd ethrex-replay/ethrex_replay_web

# Install dependencies and setup database
make setup

# Start the Phoenix server
make run
```

Now visit [http://localhost:4000](http://localhost:4000) in your browser.

### Development

```bash
# Start with interactive shell
make dev

# Run tests
make test

# Format code
make format

# Check formatting and compile with warnings as errors
make check
```

### Available Make Commands

| Command | Description |
|---------|-------------|
| `make setup` | Install dependencies and setup database (first time) |
| `make run` | Start the Phoenix server |
| `make dev` | Start server with interactive Elixir shell (iex) |
| `make deps` | Install/update dependencies |
| `make compile` | Compile the application |
| `make test` | Run tests |
| `make format` | Format code |
| `make check` | Run formatter check and compile with warnings as errors |
| `make clean` | Remove build artifacts |
| `make reset` | Reset database (drop and recreate) |
| `make assets` | Build assets (CSS/JS) |
| `make prod-build` | Build for production |
| `make prod-release` | Create production release |

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_PATH` | SQLite database location | `./ethrex_replay_web_dev.db` |
| `PHX_HOST` | Hostname for production | `localhost` |
| `PORT` | HTTP port | `4000` |
| `SECRET_KEY_BASE` | Phoenix secret key | (generated) |

### Application Config

The application can be configured in `config/`:

```elixir
# config/dev.exs
config :ethrex_replay_web, EthrexReplayWebWeb.Endpoint,
  http: [ip: {127, 0, 0, 1}, port: 4000]

# Set the ethrex-replay project directory
config :ethrex_replay_web, :project_dir, "/path/to/ethrex-replay"
```

## Architecture

### OTP Supervision Tree

```
Application
├── Telemetry
├── Repo (SQLite)
├── PubSub
├── Jobs.JobRegistry (Registry)
├── Jobs.JobSupervisor (DynamicSupervisor)
│   └── Jobs.JobServer (GenServer per job)
├── Jobs.JobQueue (GenServer)
└── Endpoint (Phoenix)
```

### Key Modules

| Module | Purpose |
|--------|---------|
| `EthrexReplayWeb.Jobs.JobQueue` | Manages job queue, ensures one job at a time |
| `EthrexReplayWeb.Jobs.JobServer` | Executes cargo commands via Ports |
| `EthrexReplayWeb.Runner.CommandBuilder` | Builds cargo CLI commands |
| `EthrexReplayWeb.System.HardwareInfo` | Detects system hardware |
| `EthrexReplayWebWeb.DashboardLive` | Main dashboard LiveView |
| `EthrexReplayWebWeb.JobLive` | Job details with real-time logs |

### Data Flow

1. User submits job configuration via LiveView form
2. `JobQueue` creates job record in SQLite and queues it
3. `JobQueue` starts `JobServer` via `JobSupervisor`
4. `JobServer` executes cargo command using Erlang Port
5. Output is streamed via PubSub to connected LiveViews
6. Results are persisted to database on completion

## Pages

### Dashboard (`/`)

The main page for creating new proof generation jobs:

- **Server Specs** - Quick overview of CPU, GPU, RAM
- **Configuration Form** - All job parameters
- **Command Preview** - Shows the exact cargo command
- **Recent Jobs** - Quick access to recent jobs
- **Current Job** - Status of running job (if any)

### Job View (`/jobs/:id`)

Detailed view for a specific job:

- **Status Badge** - Visual status indicator
- **Command** - Full command that was executed
- **Logs** - Real-time log viewer with auto-scroll
- **Results** - Execution time, proving time, gas used
- **Error Display** - Error message and exit code (if failed)

### History (`/history`)

Browse and filter all jobs:

- **Filters** - All, Running, Completed, Failed, Pending
- **Table View** - ZKVM, Action, Block, Duration, Created
- **Quick Navigation** - Click to view job details

### System (`/system`)

Hardware information and compatibility:

- **CPU** - Model and core count
- **GPU** - Model, VRAM, availability
- **RAM** - Total system memory
- **ZKVM Compatibility** - Status of each ZKVM
- **GPU Support Matrix** - Which ZKVMs support GPU

## API

The application uses Phoenix LiveView for real-time updates. There is no REST API, but the following PubSub topics are used internally:

| Topic | Messages |
|-------|----------|
| `jobs` | `{:job_created, job}`, `{:job_updated, job}`, `{:job_finished, job_id}` |
| `job:{id}` | `{:job_log, job_id, line}`, `{:job_status, job_id, status}` |

## Supported ZKVMs

| ZKVM | Status | CPU | GPU (Prove only) |
|------|--------|-----|------------------|
| SP1 | Supported | Yes | Yes (CUDA) |
| RISC0 | Supported | Yes | Yes (CUDA) |
| OpenVM | Experimental | Yes | No |
| ZisK | Experimental | Yes | No |
| Jolt | Coming Soon | - | - |
| Nexus | Coming Soon | - | - |
| Pico | Coming Soon | - | - |
| Ziren | Coming Soon | - | - |

> **Note:** GPU acceleration is only used for proving, not execution. The `gpu` feature flag is automatically added to the cargo command only when action is "prove".

## Production Deployment

### Building a Release

```bash
# Set environment variables
export SECRET_KEY_BASE=$(mix phx.gen.secret)
export DATABASE_PATH=/var/lib/ethrex_replay/prod.db

# Build and create release
make prod-release
```

### Running the Release

```bash
# Start the server
_build/prod/rel/ethrex_replay_web/bin/ethrex_replay_web start

# Or run migrations and start
_build/prod/rel/ethrex_replay_web/bin/ethrex_replay_web eval "EthrexReplayWeb.Release.migrate"
_build/prod/rel/ethrex_replay_web/bin/ethrex_replay_web start
```

### Docker (Coming Soon)

```dockerfile
# Dockerfile will be added in a future update
```

## Troubleshooting

### Common Issues

**"cargo not found"**
- Ensure Rust is installed: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Ensure cargo is in PATH: `source $HOME/.cargo/env`

**"GPU not detected"**
- For NVIDIA: Install CUDA toolkit and ensure `nvidia-smi` works
- For macOS: GPU detection uses `system_profiler`

**"Job times out"**
- Default timeout is 1 hour
- Proof generation can take significant time for large blocks
- Consider using GPU acceleration for faster proving

**"Database errors"**
- Ensure SQLite3 is installed
- Run `mix ecto.reset` to recreate the database

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is part of [ethrex-replay](https://github.com/lambdaclass/ethrex-replay) by Lambda Class.

## Links

- [ethrex-replay Repository](https://github.com/lambdaclass/ethrex-replay)
- [Phoenix Framework](https://www.phoenixframework.org/)
- [Phoenix LiveView](https://hexdocs.pm/phoenix_live_view)
- [ethproofs.org](https://ethproofs.org) (Design inspiration)
