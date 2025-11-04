# GUI

To launch the GUI authentication the suggested method is:

```sh
weston --shell_kiosk-shell.so -- polyauth-gui
```

for this to work as intended ensure you are using greetd together with seatd and that the user spawning greetd is part of the seat group. as well as having access to at least one drm device and one input device.