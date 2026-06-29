#!/usr/bin/env bash
set -euo pipefail

# Probe the noVNC absolute mouse mapping used by voxygen/src/window.rs.
# This does not build, deploy, upload, or launch the game.

pan_sensitivity="${PAN_SENSITIVITY:-100}"
deadzone="${GLITCH_VNC_ABSOLUTE_MOUSE_DEADZONE:-1.8}"
x_scale="${GLITCH_VNC_ABSOLUTE_MOUSE_X_SCALE:-0.015}"
y_scale="${GLITCH_VNC_ABSOLUTE_MOUSE_Y_SCALE:-0.006}"
max_x="${GLITCH_VNC_ABSOLUTE_MOUSE_MAX_DELTA:-48}"
max_y="${GLITCH_VNC_ABSOLUTE_MOUSE_MAX_Y_DELTA:-28}"
cursor_pan_scale="0.005"

cat <<EOF
Glitch noVNC mouse sensitivity probe

Inputs:
  PAN_SENSITIVITY=${pan_sensitivity}
  GLITCH_VNC_ABSOLUTE_MOUSE_DEADZONE=${deadzone}
  GLITCH_VNC_ABSOLUTE_MOUSE_X_SCALE=${x_scale}
  GLITCH_VNC_ABSOLUTE_MOUSE_Y_SCALE=${y_scale}
  GLITCH_VNC_ABSOLUTE_MOUSE_MAX_DELTA=${max_x}
  GLITCH_VNC_ABSOLUTE_MOUSE_MAX_Y_DELTA=${max_y}
  CURSOR_PAN_SCALE=${cursor_pan_scale} radians per cursor-pan unit

EOF

awk \
  -v pan_sensitivity="$pan_sensitivity" \
  -v deadzone="$deadzone" \
  -v x_scale="$x_scale" \
  -v y_scale="$y_scale" \
  -v max_x="$max_x" \
  -v max_y="$max_y" \
  -v cursor_pan_scale="$cursor_pan_scale" '
BEGIN {
    pi = atan2(0, -1)
    pan = pan_sensitivity / 100.0

    print "Sample X movement:"
    printf "%8s %12s %14s %14s\n", "dx_px", "cursor_pan", "rotation_rad", "rotation_deg"
    for (i = 1; i <= 6; i++) {
        dx = (i == 1 ? 2 : i == 2 ? 5 : i == 3 ? 10 : i == 4 ? 20 : i == 5 ? 32 : 48)
        if (dx > max_x) {
            printf "%8.1f %12s %14s %14s\n", dx, "ignored", "ignored", "ignored"
            continue
        }
        pan_dx = dx > deadzone ? dx : 0
        cursor_pan = pan_dx * x_scale * pan
        rotation_rad = cursor_pan * cursor_pan_scale
        rotation_deg = rotation_rad * 180.0 / pi
        printf "%8.1f %12.4f %14.6f %14.4f\n", dx, cursor_pan, rotation_rad, rotation_deg
    }

    print ""
    print "Sample Y movement:"
    printf "%8s %12s %14s %14s\n", "dy_px", "cursor_pan", "rotation_rad", "rotation_deg"
    for (i = 1; i <= 6; i++) {
        dy = (i == 1 ? 2 : i == 2 ? 5 : i == 3 ? 10 : i == 4 ? 16 : i == 5 ? 24 : 28)
        if (dy > max_y) {
            printf "%8.1f %12s %14s %14s\n", dy, "ignored", "ignored", "ignored"
            continue
        }
        pan_dy = dy > deadzone ? dy : 0
        cursor_pan = pan_dy * y_scale * pan
        rotation_rad = cursor_pan * cursor_pan_scale
        rotation_deg = rotation_rad * 180.0 / pi
        printf "%8.1f %12.4f %14.6f %14.4f\n", dy, cursor_pan, rotation_rad, rotation_deg
    }

    print ""
    print "Iframe/top-edge recenter scenario:"
    print "  Event 1 at top edge seeds the last real position: no camera pan."
    print "  Event 2 is a center/recenter warp: ignored, last real position preserved."
    print "  Event 3 is a small real iframe movement near the top edge: small camera pan."
    last_x = 640
    last_y = 8
    center_x = 640
    center_y = 360
    real_x = 645
    real_y = 10
    warp_dx = center_x - last_x
    warp_dy = center_y - last_y
    real_dx = real_x - last_x
    real_dy = real_y - last_y
    printf "  recenter warp dx=%0.1f dy=%0.1f => %s\n", warp_dx, warp_dy, (warp_dx > max_x || warp_dy > max_y ? "ignored" : "would pan")
    pan_dx = real_dx > deadzone ? real_dx : 0
    pan_dy = real_dy > deadzone ? real_dy : 0
    cursor_pan_x = pan_dx * x_scale * pan
    cursor_pan_y = pan_dy * y_scale * pan
    rotation_deg_x = cursor_pan_x * cursor_pan_scale * 180.0 / pi
    rotation_deg_y = cursor_pan_y * cursor_pan_scale * 180.0 / pi
    printf "  real move dx=%0.1f dy=%0.1f => x=%0.4f deg, y=%0.4f deg\n", real_dx, real_dy, rotation_deg_x, rotation_deg_y

    print ""
    print "Tuning hint:"
    print "  Cut GLITCH_VNC_ABSOLUTE_MOUSE_X_SCALE / Y_SCALE in half to halve camera movement."
    print "  Example: GLITCH_VNC_ABSOLUTE_MOUSE_X_SCALE=0.007 GLITCH_VNC_ABSOLUTE_MOUSE_Y_SCALE=0.003"
}'
