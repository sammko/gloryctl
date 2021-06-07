# gloryctl


This project is an implementation of the vendor-specific HID protocol in use
by [Glorious](https://www.pcgamingrace.com) mice used to configure parameters
such as DPI profiles, LED effects and macros. Most features are implemented at
this point.

## Motivation

The official program to change these parameters supplied by the vendor is proprietary
and only available for Microsoft Windows. I also used this as an opportunity to learn
more about the Rust programming language.

## The Protocol

The protocol itself, reverse-engineered using a USB capture tool to inspect
what the official program does, uses HID feature reports to communicate with the firmware
running on the mouse. The mouse is a composite USB device consisting of a standard HID
mouse on interface 0 and a keyboard with extra reports on interface 1.

The descriptors of the 2 used feature reports are as follows

	  FEATURE(4)[FEATURE]
	    Field(0)
	      Application(ff00.0001)
	      Usage(519)
			[519 fields with usage ff00.0000]
	      Logical Minimum(0)
	      Logical Maximum(255)
	      Report Size(8)
	      Report Count(519)
	      Report Offset(0)
	      Flags( Variable Absolute )
	  FEATURE(5)[FEATURE]
	    Field(0)
	      Application(ff00.0001)
	      Usage(5)
	        ff00.0000
	        ff00.0000
	        ff00.0000
	        ff00.0000
	        ff00.0000
	      Logical Minimum(0)
	      Logical Maximum(255)
	      Report Size(8)
	      Report Count(5)
	      Report Offset(0)
	      Flags( Variable Absolute )

We have two feature reports to work with, ID 4 which is 519 octets in size and
ID 5 which is only 5 octets. It turns out that a mechanism reminiscent of
bank-switching is in use. First, the host (USB host, in this case the userspace
application) sends an identifier to the mouse via report 5. The first octet in
the buffer denotes the report ID itself (5 in this case) and the second octet
is the selected command ID.

The firmware remembers the selected command and processes further read requests
accordingly. As of now, the following commands are known:

    HW_CMD_VER  = 0x01
    HW_CMD_CONF = 0x11
    HW_CMD_MAP  = 0x12
    HW_CMD_DEBOUNCE = 0x1a

### `HW_CMD_VER`

This command is used to get the version of the firmware currently running on the mouse.
It is read-only and re-uses report ID 5. So the entire communication is as follows:

1. The host selects command ID 1, using `send_feature_report([5, 1, 0, 0, 0, 0])`
   (The first octet is the report ID itself)
2. The host requests a read from the same report id: `get_feature_report(5)`
3. The mouse returns a buffer containing: The report ID again, the command ID
   untouched and the remaining 4 octets containing the version string in ASCII.
   For example `"V103"`.

### `HW_CMD_CONF`

This is the main configuration command. It uses the big (519 octet) report, but
not in its entirety, only the first 131 octets are in use. So again, the host
selects a command using report 5: `send_feature_report([5, 17, 0, 0, 0, 0])`
and then either reads report 4 or writes it. (In reality, when writing, sending
the short feature report is not necessary, as the firmware can figure out where
the write is supposed to go based on the contents of the buffer. It is obviously needed
for reading though.)

A buffer similar to this might be returned by the mouse.

	04 11 00 00 00 00 06 00 64 06 04 23 f2 04 05 05 
	05 06 06 07 07 00 00 00 00 00 00 00 00 c0 00 c0 
	ff ff ff ff 00 00 00 ff 00 ff 00 ff ff ff ff 00 
	00 00 00 00 00 00 41 00 40 ff 00 00 42 03 ff 00 
	00 00 ff 00 00 00 ff 00 00 00 00 00 00 00 00 00 
	00 00 00 42 42 00 00 00 00 00 00 00 00 00 00 00 
	00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 
	00 00 00 00 42 ff 00 00 00 ff 00 00 42 02 ff 00 
	00 01 00 

When writing, additionally, the `x[4]` value is set to decimal 123. Let's
dissect this structure:

The header:

	04    - the report ID
	11    - the command ID
	00    -
	00    - 0 when reading, 123 when writing
	00 00 -
	06    - 06 when reading, 0 when writing, but writing 6 makes no difference
	00 64 -
	06    - sensor ID
	04    - pair of nibbles, (xy_indep, poll_rate)
	23    - pair (current_profile, enabled_profile_count)
	f2    - enabled profile mask. 0 bit means enabled, 1 means disabled. lsb first

While the official software only allows for configuring 6 DPI profiles, the
mouse supports 8 perfectly well. When the `enabled_profile_count` value is set
to more than 8, the cycle length works properly, but the profiles outside the
basic 8 do not behave very well. However, who needs more than 8 DPI profiles
anyway.

DPI Profiles:

	04 05 05 05 06 06 07 07 00 00 00 00 00 00 00 00
	- DPI profile array of size 8.
	  If xy_indep is true, each profile is 2 octets (the DPI in X and Y axes). Otherwise,
      each profile is 1 octet and they are packed densely, so the second half of the array
	  is ignored.
	
	c0 00 c0 
	ff ff ff
	ff 00 00
	00 ff 00
	ff 00 ff
	ff ff ff
	00 00 00
	00 00 00
	- Colors for the 8 available DPI profiles. RGB order.

Next are the RGB effects. There is a common occurrence among the effects, the
so-called "BS" byte which stands for Brightness and Speed. The upper 4 bits of
the byte are brightness and the lower are speed of the effect. Not all effects
respect both of these parameters, but the BS byte is present in all of them. To
find out which effects support speed or brightness, see the structures defined
in `src/device.rs`

Some of these effects are not available in the original software at all. Those are:

 - ID: 6, ConstantRgb (each LED gets its own static color)
 - ID: 8, Random (randomly changing colors)

The RGB effects:

	00
	- current light effect. Enum is in the code.
	
	41 00
	- config for Glorious effect. First byte is "BS", second byte is boolean direction
	
	40 ff 00 00
	- Single color effect. "BS" byte and color in RBG, not RGB.
	
	42 03 
    ff 00 00
    00 ff 00
    00 00 ff
    00 00 00
    00 00 00
    00 00 00
    00 00 00
	- Breathing effect. "BS" byte, then number of colors(n) to cycle and then 7-long array
	  of RBG color values. The first n are cycled.
	      
	42
	- Tail effect. "BS" byte.
	
	42
    - Seamless Breathing. "BS" byte.
    
	00
    00 00 00
    00 00 00
    00 00 00
    00 00 00
    00 00 00
    00 00 00
    - Constant RGB. First byte is always 0 and has no effect (probably a degenerate "BS"
      byte with both parameters ignored). Array of 6 RBG colors.
      The LED strips on the mouse have 6 LEDs each and this sets the colors individually
      for each LED. Both strips are always the same though.
	      
	00 00 00 00 00 00 00 00 00 00 00 00
    - 12 unknown bytes. The mouse originally had some ff bytes also, but I overwrote them
      and nothing changed to my knowledge.
	
	42 ff 00 00 00 ff 00
	- The Rave effect. First byte is "BS" and then 2 colors in RBG.
	
	00
	- Random effect "BS".
	
	42
	- Wave effect "BS".
	
	02 ff 00 00
    - Single color breathing "BS" and RBG color.

Trailer:

	01
	- LOD
	
	00
	- unknown

The reason I know how long the array actually is, when it is sent in a 520
octet buffer (including the first octet which is the report number) is that the
mouse actually sends a shorter buffer which is detected by the analyzer. On the
other hand, when sending the buffer to the mouse, its behaviour changes in
unknown ways (it mostly doesn't work, that is) depending on the length of the
sent buffer. The official software sends the entire 520 thing padded with
zeroes.

### `HW_CMD_MAP`

This is used to configure the button mapping -- what each physical button on the
mouse actually does. The default configuration is as follows:

    Button 1 - Left click
    Button 2 - Right click
    Button 3 - Middle click
    Button 4 - Back
    Button 5 - Forward
    Button 6 - DPI cycle

Each of these buttons can be reconfigured arbitrarily, the possibilities are
somewhat apparent from the enum I use to represent them:

    pub enum ButtonAction {
        MouseButton(MouseButton),
        Scroll(i8),
        RepeatButton {
            which: MouseButton,
            interval: u8,
            count: u8,
        },
        DpiSwitch(DpiSwitch),
        DpiLock(u16),
        MediaButton(MediaButton),
        KeyboardShortcut {
            modifiers: Modifier,
            key: u8,
        },
        Disabled,
        Macro(u8, MacroMode),
    }

Interestingly, the MouseButton and MediaButton types are bitflags, meaning that
multiple actions of the same kind can be performed simultaneously by the device,
for example a button can be configured to press left and middle click at the
same time. The binary representation of this is relatively straightforward,
and details can quickly be gleaned from the implementation of the encoder.

In summary, the report consists of a constant header, followed by several
repetitions of a structure 4 bytes in size. Each instance represents the
mapping for one button. The official software sends a buffer containing
mapping for 20 buttons, but the mouse only has 6 buttons and only uses
the first 6 entries. The first byte of each entry roughly corresponds to
the variants of the enum above. The remaining three bytes encode the
corresponding data.

Unfortunately, I have not found a way to retrieve the current configuration off
the mouse, trying to read the buffer in a manner similar to `HW_CMD_CONF` does work,
but returns the default configuration. I have once seen the mouse switch to
a different mode somehow by the official software, where it would ignore
writes to the `HW_CMD_CONF` buffer (no changes in RGB for example) and would
return the actual value of `HW_CMD_MAP` instead of the default mapping.
I have not found out how the official software does it or why.

### `HW_CMD_DEBOUNCE`

This is a simple command to change the debounce timing of the mouse.
It uses only the short report. For example a write of `[5, 0x1a, D, 0, 0, 0]`,
would configure the debounce time to `2*D ms`. Setting D to 0 and reading
the same report yields the currently configured value.

### Programmable Macros

The mouse has support for macros. Each button can be configured to execute a
macro with a given number (either once, multiple times, or repeating).

A macro is a sequence of (up to 168) events, where each event contains a state
(Up or Down, whether the key is pressed or released), a delay value
in milliseconds and each event contains the key it is describing.
A key is either a mouse button, a modifier (ctrl, etc.)
or a normal button on the keyboard (described by the USB HID usage number).

Each event is represented by a 3 byte structure. The details can be gleaned
quite easily from the implementation of the encoder.

The mouse has memory for several independent macro banks. How many? Trying to
find out by writing to them led to a bricked mouse (probably the firmware code
does not check bounds and ended up overwriting some important data).
They are numbered and can be written using the big report with a header
of `[4, 0x30, 0x02, 0, 0, 0, 0, 0, bank_number, 0, event_count]`. The same
`bank_number` is used as an identifier when configuring the button map.

To my knowledge, the macros cannot be read from the mouse.

## The Program

`gloryctl` has a simple command line interface to configure features of the
mouse. Use `gloryctl --help` to see usage information.

It should run fine on both Linux and Windows based operating systems thanks to
cross-platform support of the hidapi library, which is used for low-level
communication with the device. macOS should work as well in theory, but I have
not yet tested this claim.

`src/device.rs` contains definitions for some hardware constants, definitions
for structures used by the library and code for sending commands to the mouse.

`src/protocol/decode.rs` contains parsing routines for the binary structures
written using the `nom` crate. I find it needlessly powerful for this
application and relatively hard to understand and work with, so I might look
for a simpler alternative in the future.

`src/protocol/encode.rs` contains routines which do the inverse operation of
assembling data from the defined data types back into the byte blobs the mouse
expects. There is nothing particularly illuminating, the correct values are
just placed in the right order into a byte array.


### Example output

This is what the parsed structure might look like:

    [src/main.rs:21] conf = Config {
        header: [
            4,
            17,
            0,
            0,
            0,
            0,
            6,
            0,
            100,
        ],
        sensor_id: 6,
        dpi_axes_independent: false,
        polling_rate: Hz1000,
        dpi_current_profile: 2,
        dpi_profile_count: 3,
        dpi_profiles: [
            DpiProfile {
                enabled: true,
                value: Single(
                    4,
                ),
                color: Color {
                    r: 192,
                    g: 0,
                    b: 192,
                },
            },
            DpiProfile {
                enabled: false,
                value: Single(
                    5,
                ),
                color: Color {
                    r: 255,
                    g: 255,
                    b: 255,
                },
            },
            DpiProfile {
                enabled: true,
                value: Single(
                    5,
                ),
                color: Color {
                    r: 255,
                    g: 0,
                    b: 0,
                },
            },
            DpiProfile {
                enabled: true,
                value: Single(
                    5,
                ),
                color: Color {
                    r: 0,
                    g: 255,
                    b: 0,
                },
            },
            DpiProfile {
                enabled: false,
                value: Single(
                    6,
                ),
                color: Color {
                    r: 255,
                    g: 0,
                    b: 255,
                },
            },
            DpiProfile {
                enabled: false,
                value: Single(
                    6,
                ),
                color: Color {
                    r: 255,
                    g: 255,
                    b: 255,
                },
            },
            DpiProfile {
                enabled: false,
                value: Single(
                    7,
                ),
                color: Color {
                    r: 0,
                    g: 0,
                    b: 0,
                },
            },
            DpiProfile {
                enabled: false,
                value: Single(
                    7,
                ),
                color: Color {
                    r: 0,
                    g: 0,
                    b: 0,
                },
            },
        ],
        rgb_current_effect: Off,
        rgb_effect_parameters: EffectParameters {
            glorious: Glorious {
                speed: 1,
                direction: 0,
            },
            single_color: SingleColor {
                brightness: 4,
                color: Color {
                    r: 255,
                    g: 0,
                    b: 0,
                },
            },
            breathing: Breathing {
                speed: 2,
                count: 3,
                colors: [
                    Color {
                        r: 255,
                        g: 0,
                        b: 0,
                    },
                    Color {
                        r: 0,
                        g: 0,
                        b: 255,
                    },
                    Color {
                        r: 0,
                        g: 255,
                        b: 0,
                    },
                    Color {
                        r: 0,
                        g: 0,
                        b: 0,
                    },
                    Color {
                        r: 0,
                        g: 0,
                        b: 0,
                    },
                    Color {
                        r: 0,
                        g: 0,
                        b: 0,
                    },
                    Color {
                        r: 0,
                        g: 0,
                        b: 0,
                    },
                ],
            },
            tail: Tail {
                speed: 2,
                brightness: 4,
            },
            seamless_breathing: SeamlessBreathing {
                speed: 2,
            },
            constant_rgb: ConstantRgb {
                colors: [
                    Color {
                        r: 0,
                        g: 0,
                        b: 0,
                    },
                    Color {
                        r: 0,
                        g: 0,
                        b: 0,
                    },
                    Color {
                        r: 0,
                        g: 0,
                        b: 0,
                    },
                    Color {
                        r: 0,
                        g: 0,
                        b: 0,
                    },
                    Color {
                        r: 0,
                        g: 0,
                        b: 0,
                    },
                    Color {
                        r: 0,
                        g: 0,
                        b: 0,
                    },
                ],
            },
            rave: Rave {
                speed: 2,
                brightness: 4,
                colors: [
                    Color {
                        r: 255,
                        g: 0,
                        b: 0,
                    },
                    Color {
                        r: 0,
                        g: 0,
                        b: 255,
                    },
                ],
            },
            random: Random {
                speed: 0,
            },
            wave: Wave {
                speed: 2,
                brightness: 4,
            },
            single_breathing: SingleBreathing {
                speed: 2,
                color: Color {
                    r: 255,
                    g: 0,
                    b: 0,
                },
            },
        },
        unknown: (
            [
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ],
            0,
        ),
        lod: 1,
    }
