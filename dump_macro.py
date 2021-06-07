#!/bin/python
import sys
from itertools import zip_longest
import enum

class Buttons(enum.IntFlag):
    L = 0x01
    R = 0x02
    M = 0x04
    B = 0x08
    F = 0x10

class Modifier(enum.IntFlag):
    Ctrl = 0x01
    Shift = 0x02
    Alt = 0x04
    Win = 0x08

def grouper(n, iterable, fillvalue=None):
    "grouper(3, 'ABCDEFG', 'x') --> ABC DEF Gxx"
    args = [iter(iterable)] * n
    return zip_longest(fillvalue=fillvalue, *args)

with open(sys.argv[1], "rb") as fd:
    raw = fd.read()
    assert raw[:8] == b'\x04\x30\x02\x00\x00\x00\x00\x00'
    print("bank num:", raw[8])
    # unk raw[9], maybe LE bank num MSB?
    assert raw[9] == 0
    evcount = raw[10]
    print("event count:", evcount)
    data = raw[11:11+evcount*3]

    for i, ev in enumerate(grouper(3, data)):
        head = (ev[0] >> 4)
        duration = ev[1] | (ev[0] & 0xf) << 8
        key = ev[2]
        type = head & 7
        down_or_up = (head >> 3) & 1
        direction = ["D", "U"][down_or_up]
        typename = {5: "Keybo", 1: "Mouse", 6: "Modif"}.get(type, "Unknown")
        s = ""
        if type == 1:
            btn = Buttons(key)
            s = f"btn {btn}"
        elif type == 6:
            m = Modifier(key)
            s = f"mod {m}"
        elif type == 5:
            s = f"key {key}" 
        print(f"{i:3}: T {duration:4}, type {typename}, dir {direction}, {s}")
  
    # rest should be zero
    assert all([x == 0 for x in raw[11+evcount*3:]])
