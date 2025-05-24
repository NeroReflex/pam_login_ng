#!/bin/bash

# TODO: close gnome
gnome-session-quit --no-prompt

# Check if org.kde.Shutdown is available
# and if so, use that to close plasma
if qdbus | grep -q "org.kde.Shutdown"; then
    qdbus org.kde.Shutdown /Shutdown logout
fi

# restart login_ng
login_ng-sessionctl restart
