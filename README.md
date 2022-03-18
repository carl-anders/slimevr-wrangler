# SlimeVR Wrangler

Use Joy-Con's as SlimeVR trackers, enabling you to make a full body system with your existing devices. Combine with DIY SlimeVR trackers, or phones using owoTrack.

![Screenshot of the app running and tracking a single Joy-Con](screenshot.png)

## Setup
You need bluetooth on your computer.
* Download and set up [SlimeVR](https://docs.slimevr.dev/slimevr-setup.html)
* Download [SlimeVR Wrangler](https://github.com/carl-anders/slimevr-wrangler/releases/latest/download/slimevr-wrangler.exe)
* Start both the SlimeVR server and SlimeVR Wrangler 
* Connect your Joy-Con trackers to the computer ([Guide for Windows](https://www.digitaltrends.com/gaming/how-to-connect-a-nintendo-switch-controller-to-a-pc/))
* Make sure the SlimeVR server is running, then press "Search for Joycons" inside SlimeVR Wrangler
* The Joy-Con should show up in the window!
* Follow the SlimeVR documentation to set up the new tracker, with the direction below:

### Mounting

Attach the Joy-Con's in the direction that works best for you, use the SlimeVR guide to see the positions on your body.

Keep the joystick pointed outwards, it should not poke into your skin.

After connecting the Joy-Con's in the program, rotate them in the program to be the same rotation as they are if you are standing up.

## Issues

Many! This is a **alpha** version, and there's no guarantees about anything.

* Rotation tracking is bad! - Yup, sorry. In the future there will be settings to help fine tune the tracking. I suggest binding a button to reset.
* It stops tracking when I turn around! - Bluetooth does not have a good range, you might have better luck with a different bluetooth adapter.
* Probably more.

### My Joy-Con's are connected in the Windows bluetooth menu but won't show up!

This is a problem that might be related to a newer Windows update. Try this, and it might fix it:
* Go to the Windows Setting app -> Bluetooth & other devices.
* Press on the Joy-Con that won't connect. Press "Remove device".
* Pair the device again. It should now show up.

# License
GPLv3

If you contribute to this, you also agree to license your contributions with MIT License & Apache License, Version 2.0. This is because I might replace the GPL dependency in the future with something else.
