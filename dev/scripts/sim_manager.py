#!/usr/bin/env python3
"""
Simulation Manager for Termion Development

Manages simulated MinKNOW devices for development and testing.

Usage:
    python3 sim_manager.py status
    python3 sim_manager.py create [device_name] [--type minion|promethion|p2]
    python3 sim_manager.py remove [device_name]
    python3 sim_manager.py playback <bulk_file> [device_name] [--loop]
    python3 sim_manager.py setup <bulk_file> [device_name]
"""

import argparse
import sys
import time
from pathlib import Path

try:
    from minknow_api.manager import Manager
    from minknow_api.manager_pb2 import SimulatedDeviceType
    from minknow_api import acquisition_pb2
except ImportError:
    print("Error: minknow_api not installed")
    print("Install with: pip install minknow_api")
    print("Or activate the dev venv: source dev/.venv/bin/activate")
    sys.exit(1)


DEFAULT_HOST = "localhost"
DEFAULT_PORT = 9501
DEFAULT_DEVICE = "MS00001"

DEVICE_TYPES = {
    "minion": SimulatedDeviceType.SIMULATED_MINION,
    "promethion": SimulatedDeviceType.SIMULATED_PROMETHION,
    "p2": SimulatedDeviceType.SIMULATED_P2,
}


def get_manager(host: str = DEFAULT_HOST, port: int = DEFAULT_PORT) -> Manager:
    """Connect to MinKNOW manager."""
    try:
        return Manager(host=host, port=port)
    except Exception as e:
        print(f"Error connecting to MinKNOW at {host}:{port}")
        print(f"  {e}")
        print()
        print("Is MinKNOW running? Try: sudo systemctl start minknow")
        sys.exit(1)


def cmd_status(args):
    """Show MinKNOW status and positions."""
    m = get_manager(args.host, args.port)
    
    positions = list(m.flow_cell_positions())
    
    print("MinKNOW Status")
    print(f"  Host: {args.host}:{args.port}")
    print(f"  Connected: Yes")
    print(f"  Positions: {len(positions)}")
    
    if positions:
        print()
        print("Devices:")
        for pos in positions:
            # Get more details if possible
            try:
                conn = pos.connect()
                info = conn.acquisition.get_acquisition_info()
                # Map state enum to readable name
                state_names = {
                    0: "READY",
                    1: "STARTING", 
                    2: "SEQUENCING",
                    3: "IDLE",
                    4: "FINISHING",
                    5: "FINISHED",
                }
                state_str = state_names.get(info.state, f"STATE_{info.state}")
            except Exception:
                state_str = str(pos.state)
            
            print(f"  - {pos.name}: {state_str}")


def cmd_create(args):
    """Create a simulated device."""
    m = get_manager(args.host, args.port)
    device_name = args.device_name or DEFAULT_DEVICE
    device_type = DEVICE_TYPES.get(args.type, SimulatedDeviceType.SIMULATED_MINION)
    
    # Check if already exists
    existing = [p.name for p in m.flow_cell_positions()]
    if device_name in existing:
        print(f"Device {device_name} already exists")
        return
    
    print(f"Creating simulated {args.type}: {device_name}...")
    try:
        m.add_simulated_device(device_name, device_type)
        time.sleep(2)  # Wait for device to initialize
        print(f"  Success!")
        print()
        print("Next steps:")
        print("  1. Open MinKNOW GUI: http://localhost:9501")
        print(f"  2. Select position: {device_name}")
        print("  3. Start a sequencing run")
    except Exception as e:
        print(f"Error creating device: {e}")
        sys.exit(1)


def cmd_remove(args):
    """Remove a simulated device."""
    m = get_manager(args.host, args.port)
    device_name = args.device_name or DEFAULT_DEVICE
    
    # Check if exists
    existing = [p.name for p in m.flow_cell_positions()]
    if device_name not in existing:
        print(f"Device {device_name} does not exist")
        return
    
    print(f"Removing simulated device: {device_name}...")
    try:
        m.remove_simulated_device(device_name)
        print("  Success!")
    except Exception as e:
        print(f"Error removing device: {e}")
        sys.exit(1)


def cmd_playback(args):
    """Configure bulk file playback for a device."""
    m = get_manager(args.host, args.port)
    device_name = args.device_name or DEFAULT_DEVICE
    bulk_file = Path(args.bulk_file).resolve()
    
    # Validate bulk file exists
    if not bulk_file.exists():
        print(f"Error: Bulk file not found: {bulk_file}")
        sys.exit(1)
    
    # Find the position
    positions = {p.name: p for p in m.flow_cell_positions()}
    if device_name not in positions:
        print(f"Error: Device {device_name} not found")
        print(f"  Available: {list(positions.keys()) or 'none'}")
        print()
        print("Create a device first: python3 sim_manager.py create")
        sys.exit(1)
    
    pos = positions[device_name]
    
    # Configure playback
    print(f"Configuring playback for {device_name}...")
    print(f"  Bulk file: {bulk_file}")
    print(f"  Mode: {'LOOP' if args.loop else 'SINGLE_RUN'}")
    
    try:
        conn = pos.connect()
        
        mode = (
            acquisition_pb2.SetSignalReaderRequest.SourceFileMode.LOOP
            if args.loop
            else acquisition_pb2.SetSignalReaderRequest.SourceFileMode.SINGLE_RUN
        )
        
        conn.acquisition.set_signal_reader(
            reader=acquisition_pb2.SetSignalReaderRequest.SignalReaderType.HDF5,
            hdf_source=str(bulk_file),
            hdf_mode=mode,
        )
        
        # Verify
        reader = conn.acquisition.get_signal_reader()
        print(f"  Playback source: {reader.playback_source}")
        print()
        print("Success! Start a run from MinKNOW GUI.")
        
    except Exception as e:
        print(f"Error configuring playback: {e}")
        print()
        print("Common causes:")
        print("  - File not readable by minknow user")
        print("  - File is not a valid bulk FAST5")
        print()
        print("Fix permissions:")
        print(f"  sudo chown minknow:minknow {bulk_file}")
        print("  OR")
        print(f"  chmod o+r {bulk_file}")
        sys.exit(1)


def cmd_setup(args):
    """Full setup: create device and configure playback."""
    # Create device if needed
    m = get_manager(args.host, args.port)
    device_name = args.device_name or DEFAULT_DEVICE
    
    existing = [p.name for p in m.flow_cell_positions()]
    if device_name in existing:
        print(f"Device {device_name} already exists, removing...")
        try:
            m.remove_simulated_device(device_name)
            time.sleep(3)
        except Exception as e:
            print(f"  Warning: {e}")
    
    # Create fresh device
    print(f"Creating simulated MinION: {device_name}...")
    try:
        m.add_simulated_device(device_name, SimulatedDeviceType.SIMULATED_MINION)
        time.sleep(2)
    except Exception as e:
        if "already in use" not in str(e):
            print(f"Error: {e}")
            sys.exit(1)
    
    # Configure playback
    args.loop = getattr(args, 'loop', False)
    cmd_playback(args)


def main():
    parser = argparse.ArgumentParser(
        description="Manage simulated MinKNOW devices for Termion development"
    )
    parser.add_argument(
        "--host", default=DEFAULT_HOST, help=f"MinKNOW host (default: {DEFAULT_HOST})"
    )
    parser.add_argument(
        "--port", type=int, default=DEFAULT_PORT, 
        help=f"MinKNOW port (default: {DEFAULT_PORT})"
    )
    
    subparsers = parser.add_subparsers(dest="command", required=True)
    
    # status
    status_parser = subparsers.add_parser("status", help="Show MinKNOW status")
    status_parser.set_defaults(func=cmd_status)
    
    # create
    create_parser = subparsers.add_parser("create", help="Create simulated device")
    create_parser.add_argument(
        "device_name", nargs="?", default=DEFAULT_DEVICE,
        help=f"Device name (default: {DEFAULT_DEVICE})"
    )
    create_parser.add_argument(
        "--type", choices=["minion", "promethion", "p2"], default="minion",
        help="Device type (default: minion)"
    )
    create_parser.set_defaults(func=cmd_create)
    
    # remove
    remove_parser = subparsers.add_parser("remove", help="Remove simulated device")
    remove_parser.add_argument(
        "device_name", nargs="?", default=DEFAULT_DEVICE,
        help=f"Device name (default: {DEFAULT_DEVICE})"
    )
    remove_parser.set_defaults(func=cmd_remove)
    
    # playback
    playback_parser = subparsers.add_parser(
        "playback", help="Configure bulk file playback"
    )
    playback_parser.add_argument("bulk_file", help="Path to bulk FAST5 file")
    playback_parser.add_argument(
        "device_name", nargs="?", default=DEFAULT_DEVICE,
        help=f"Device name (default: {DEFAULT_DEVICE})"
    )
    playback_parser.add_argument(
        "--loop", action="store_true", help="Loop playback continuously"
    )
    playback_parser.set_defaults(func=cmd_playback)
    
    # setup
    setup_parser = subparsers.add_parser(
        "setup", help="Full setup: create device + configure playback"
    )
    setup_parser.add_argument("bulk_file", help="Path to bulk FAST5 file")
    setup_parser.add_argument(
        "device_name", nargs="?", default=DEFAULT_DEVICE,
        help=f"Device name (default: {DEFAULT_DEVICE})"
    )
    setup_parser.add_argument(
        "--loop", action="store_true", help="Loop playback continuously"
    )
    setup_parser.set_defaults(func=cmd_setup)
    
    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
