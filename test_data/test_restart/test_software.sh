#!/bin/bash
if [ ! -f "f1" ]; then
    touch "f1"
elif [ ! -f "f2" ]; then
    touch "f2"
else
    touch "f3"
fi

sleep 1
