#!/bin/bash

# Exit on error
set -e

# Configuration
SERVICE_NAME="mox-adb"
SERVICE_FILE="mox-adb.service"
SYSTEMD_DIR="/etc/systemd/system"

# Get current user and directory
CURRENT_USER=$(whoami)
CURRENT_DIR=$(pwd)

# Check if running as root (we shouldn't run build as root usually, but installation needs root)
if [ "$EUID" -eq 0 ]; then
  echo "Please run this script as a normal user (not root/sudo). Sudo will be requested when needed."
  exit 1
fi

echo "Installing $SERVICE_NAME..."
echo "User: $CURRENT_USER"
echo "Directory: $CURRENT_DIR"

# 1. Build release version
echo "Building release version..."
cargo build --release

# 2. Check if build was successful
if [ ! -f "target/release/mox-adb" ]; then
    echo "Error: Build failed. target/release/mox-adb not found."
    exit 1
fi

# 3. Create temporary service file with correct paths
echo "Configuring service file..."
TEMP_SERVICE_FILE="/tmp/$SERVICE_FILE"
cp $SERVICE_FILE $TEMP_SERVICE_FILE

# Replace placeholders
sed -i "s|{{USER}}|$CURRENT_USER|g" $TEMP_SERVICE_FILE
sed -i "s|{{WORKDIR}}|$CURRENT_DIR|g" $TEMP_SERVICE_FILE

# 4. Install service file
echo "Installing systemd service..."
sudo cp $TEMP_SERVICE_FILE $SYSTEMD_DIR/$SERVICE_FILE
sudo chmod 644 $SYSTEMD_DIR/$SERVICE_FILE
rm $TEMP_SERVICE_FILE

# 5. Reload systemd
echo "Reloading systemd daemon..."
sudo systemctl daemon-reload

# 6. Enable and start service
echo "Enabling and starting service..."
sudo systemctl enable $SERVICE_NAME
sudo systemctl restart $SERVICE_NAME

# 7. Check status
echo "Checking service status..."
sudo systemctl status $SERVICE_NAME --no-pager

echo "Installation complete!"
echo "You can check logs with: sudo journalctl -u $SERVICE_NAME -f"
