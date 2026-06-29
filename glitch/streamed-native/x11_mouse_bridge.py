#!/usr/bin/env python3
import asyncio
import base64
import ctypes
import hashlib
import json
import math
import os
import struct
from urllib.parse import parse_qs, urlparse


DISPLAY_NAME = os.environ.get("DISPLAY", ":99").encode()
HOST = "127.0.0.1"
PORT = int(os.environ.get("GLITCH_X11_MOUSE_BRIDGE_PORT", "6090"))
MAX_DELTA = float(os.environ.get("GLITCH_X11_MOUSE_MAX_DELTA", "80"))

x11 = ctypes.CDLL("libX11.so.6")
xtst = ctypes.CDLL("libXtst.so.6")
x11.XOpenDisplay.argtypes = [ctypes.c_char_p]
x11.XOpenDisplay.restype = ctypes.c_void_p
x11.XFlush.argtypes = [ctypes.c_void_p]
xtst.XTestFakeRelativeMotionEvent.argtypes = [
    ctypes.c_void_p,
    ctypes.c_int,
    ctypes.c_int,
    ctypes.c_ulong,
]

display = x11.XOpenDisplay(DISPLAY_NAME)
if not display:
    raise SystemExit(f"could not open X display {DISPLAY_NAME!r}")


def clamp(value):
    try:
        value = float(value)
    except Exception:
        return 0
    if not math.isfinite(value):
        return 0
    return int(round(max(-MAX_DELTA, min(MAX_DELTA, value))))


def move(dx, dy):
    dx = clamp(dx)
    dy = clamp(dy)
    if dx == 0 and dy == 0:
        return False
    xtst.XTestFakeRelativeMotionEvent(display, dx, dy, 0)
    x11.XFlush(display)
    return True


async def read_exact(reader, count):
    return await reader.readexactly(count)


async def handle_ws(reader, _writer):
    while True:
        try:
            b1, b2 = await read_exact(reader, 2)
        except Exception:
            return

        opcode = b1 & 0x0F
        masked = bool(b2 & 0x80)
        length = b2 & 0x7F

        if length == 126:
            length = struct.unpack("!H", await read_exact(reader, 2))[0]
        elif length == 127:
            length = struct.unpack("!Q", await read_exact(reader, 8))[0]

        mask = await read_exact(reader, 4) if masked else b""
        payload = await read_exact(reader, length) if length else b""
        if masked:
            payload = bytes(byte ^ mask[i % 4] for i, byte in enumerate(payload))

        if opcode == 8:
            return
        if opcode not in (1, 2):
            continue

        try:
            data = json.loads(payload.decode("utf-8", errors="ignore"))
            move(data.get("dx", 0), data.get("dy", 0))
        except Exception:
            continue


async def handle_client(reader, writer):
    try:
        request = await reader.readuntil(b"\r\n\r\n")
    except Exception:
        writer.close()
        return

    text = request.decode("latin1", errors="ignore")
    lines = text.split("\r\n")
    parts = lines[0].split()
    path = parts[1] if len(parts) > 1 else "/"
    headers = {}

    for line in lines[1:]:
        if ":" in line:
            key, value = line.split(":", 1)
            headers[key.strip().lower()] = value.strip()

    parsed = urlparse(path)

    if headers.get("upgrade", "").lower() == "websocket":
        key = headers.get("sec-websocket-key", "")
        accept = base64.b64encode(
            hashlib.sha1((key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").encode()).digest()
        ).decode()
        writer.write(
            (
                "HTTP/1.1 101 Switching Protocols\r\n"
                "Upgrade: websocket\r\n"
                "Connection: Upgrade\r\n"
                f"Sec-WebSocket-Accept: {accept}\r\n"
                "Cache-Control: no-store\r\n\r\n"
            ).encode()
        )
        await writer.drain()
        await handle_ws(reader, writer)
        writer.close()
        return

    if parsed.path == "/glitch-x11-mouse-delta":
        qs = parse_qs(parsed.query)
        ok = move(qs.get("dx", ["0"])[0], qs.get("dy", ["0"])[0])
        body = json.dumps({"ok": ok}).encode()
        writer.write(
            b"HTTP/1.1 200 OK\r\n"
            b"Content-Type: application/json\r\n"
            b"Cache-Control: no-store\r\n"
            b"Content-Length: "
            + str(len(body)).encode()
            + b"\r\n\r\n"
            + body
        )
        await writer.drain()
        writer.close()
        return

    body = b"ok\n"
    writer.write(
        b"HTTP/1.1 200 OK\r\n"
        b"Content-Type: text/plain\r\n"
        b"Cache-Control: no-store\r\n"
        b"Content-Length: 3\r\n\r\n"
        + body
    )
    await writer.drain()
    writer.close()


async def main():
    server = await asyncio.start_server(handle_client, HOST, PORT)
    print(f"glitch x11 mouse bridge listening on {HOST}:{PORT}", flush=True)
    async with server:
        await server.serve_forever()


asyncio.run(main())
