# Development Environment Setup

This document describes how to set up a development environment for Termion, including simulated MinKNOW devices for testing without real sequencing hardware.

## Prerequisites

1. **MinKNOW installed and running** (version 6.x+)
2. **Python 3.10+** with `minknow_api` package
3. **Rust toolchain** (rustup, cargo)
4. **Bulk FAST5 file** for playback (optional, for realistic simulation)

## Quick Start

### 1. Set Up Python Environment

The simulation scripts require the `minknow_api` Python package:

```bash
cd dev
python3 -m venv .venv
source .venv/bin/activate
pip install minknow_api
```

### 2. Verify MinKNOW Connection

```bash
source dev/.venv/bin/activate
python3 dev/scripts/sim_manager.py status
```

Expected output:
```
MinKNOW Status
  Host: localhost:9501
  Connected: Yes
  Positions: 0
```

### 3. Create a Simulated Device

```bash
python3 dev/scripts/sim_manager.py create
```

This creates a simulated MinION named `MS00001`.

### 4. (Optional) Configure Bulk File Playback

If you have a bulk FAST5 file for realistic signal playback:

```bash
python3 dev/scripts/sim_manager.py playback /path/to/bulk.fast5
```

**Note**: The file must be readable by the `minknow` user. See [Bulk File Setup](#bulk-file-setup) below.

### 5. Start a Run

1. Open MinKNOW GUI: http://localhost:9501
2. Select position `MS00001`
3. Start a sequencing run (any kit works for simulation)

### 6. Run Termion

```bash
cargo run
```

## Simulation Management

### Available Commands

```bash
# Show MinKNOW status and positions
python3 dev/scripts/sim_manager.py status

# Create simulated MinION (default: MS00001)
python3 dev/scripts/sim_manager.py create [device_name]

# Remove simulated device
python3 dev/scripts/sim_manager.py remove [device_name]

# Configure bulk file playback
python3 dev/scripts/sim_manager.py playback <bulk_file> [device_name]

# Full setup: create device + configure playback
python3 dev/scripts/sim_manager.py setup <bulk_file> [device_name]
```

### Device Types

| Type | Name Format | Channels | Command |
|------|-------------|----------|---------|
| MinION | MS##### | 512 | `create --type minion` |
| PromethION | PS##### | 3000 | `create --type promethion` |
| P2 | P2##### | 8000 | `create --type p2` |

## Bulk File Setup

Bulk FAST5 files contain raw signal data for realistic playback.

### Obtaining Bulk Files

1. **Local test data**: Check `/minknow/test-data/` for existing bulk files
2. **From previous runs**: MinKNOW can save bulk files during sequencing
3. **ONT community**: Check Nanopore Community for sample datasets
4. **Generate from FAST5**: Some tools can create bulk files from existing data

### Default Test File

A bulk file is available for development:
```
/minknow/test-data/GXB02001_20230509_1250_FAW79338_X3_sequencing_run_NA12878_B1_19382aa5_ef4362cd.fast5
```

Quick setup:
```bash
make sim-setup BULK_FILE=/minknow/test-data/GXB02001_20230509_1250_FAW79338_X3_sequencing_run_NA12878_B1_19382aa5_ef4362cd.fast5
```

### File Permissions

MinKNOW runs as the `minknow` user and needs read access:

```bash
# Option 1: Copy to MinKNOW data directory
sudo cp /path/to/bulk.fast5 /var/lib/minknow/data/
sudo chown minknow:minknow /var/lib/minknow/data/bulk.fast5

# Option 2: Make existing file world-readable
chmod o+r /path/to/bulk.fast5
chmod o+rx /path/to/containing/directory
```

### Playback Modes

| Mode | Description | Use Case |
|------|-------------|----------|
| `SINGLE_RUN` | Play once, then stop | Testing run lifecycle |
| `LOOP` | Continuously repeat | Long-running development |

Default is `SINGLE_RUN`. To use loop mode:
```bash
python3 dev/scripts/sim_manager.py playback /path/to/bulk.fast5 --loop
```

## Troubleshooting

### "Connection refused" on port 9501

MinKNOW is not running. Start it:
```bash
sudo systemctl start minknow
```

### "Permission denied" when starting playback run

The bulk file isn't readable by MinKNOW:
```bash
# Check MinKNOW user
ps aux | grep mk_manager

# Fix permissions
sudo chown minknow:minknow /path/to/bulk.fast5
```

### Simulated device stuck in bad state

Remove and recreate:
```bash
python3 dev/scripts/sim_manager.py remove MS00001
python3 dev/scripts/sim_manager.py create MS00001
```

### Run ends immediately

This can happen if:
- No bulk file configured (runs with no signal)
- Bulk file is corrupted or incompatible

Try starting the run from MinKNOW GUI which handles simulation quirks better.

### Python "ModuleNotFoundError: minknow_api"

Activate the dev virtual environment:
```bash
source dev/.venv/bin/activate
```

Or install the package:
```bash
pip install minknow_api
```

## Development Workflow

### Typical Session

```bash
# 1. Start simulated device (once per session)
source dev/.venv/bin/activate
python3 dev/scripts/sim_manager.py create

# 2. Build and run termion
cargo run

# 3. Start a run from MinKNOW GUI to generate data

# 4. Iterate on code
cargo run

# 5. Clean up when done (optional)
python3 dev/scripts/sim_manager.py remove
```

### Without Bulk File

You can develop without a bulk file — MinKNOW will simulate with synthetic data. However:
- Statistics will be minimal/zero
- Runs may behave differently than real sequencing
- Good enough for UI development and basic testing

### With Bulk File

More realistic testing:
- Real signal patterns
- Meaningful statistics
- Accurate run timing
- Better for testing charts and metrics display

## Project Structure

```
termion/
├── dev/
│   ├── README.md           # This file
│   ├── .venv/              # Python venv for minknow_api
│   └── scripts/
│       └── sim_manager.py  # Simulation management tool
├── src/
├── Cargo.toml
└── ...
```
