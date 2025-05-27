#!/bin/bash

# Start the process
#startplasma-wayland &

COMMAND="gamescope -e --steam -- steam"

WAITCMD="${COMMAND%% *}"
if [ "$WAITCMD" == "startplasma-wayland" ]; then
    WAITCMD="kwin_wayland"
fi

$COMMAND &

# Capture the PID of the last background command
PID=$!

# Function to handle SIGTERM
handle_sigterm() {
    echo "Received SIGTERM. Performing cleanup..."
    # Add any custom actions you want to perform here
    echo "Exiting the script."
    exit 0
}

# Trap SIGTERM signal and call the handler
trap 'handle_sigterm' SIGTERM

echo "waiting for $PID"

wait $PID

EXIT_STATUS=$?

echo "process $PID terminated with $EXIT_STATUS"

# Infinite loop to keep checking the process
while true; do
    echo "Checking for $WAITCMD"
    
    sleep 1
    
    # Check if plasma is still running
    if pgrep -u "$USER" "$WAITCMD" > /dev/null; then
        echo "$WAITCMD still running..."
    else
        exit $EXIT_STATUS
    fi
done
