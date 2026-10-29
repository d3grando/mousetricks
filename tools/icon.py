"""Draws icon.png, the registry icon: a mouse cursor dragging across a row of inventory slots, an item dropped into
each slot it has crossed, in a vanilla tooltip's frame (the frame LookAt's icon uses, so the mods look like one
family). 32 x 32 pixel art scaled 8 times to 256 x 256. Needs Pillow.

    python tools/icon.py [icon.png]
"""
import sys

from PIL import Image

SIZE, SCALE = 32, 8
BACKGROUND = (0x10, 0x00, 0x10, 0xF0)
FRAME_TOP = (0x50, 0x00, 0xFF, 0xFF)
FRAME_BOTTOM = (0x28, 0x00, 0x7F, 0xFF)


def lerp(a, b, t):
    return tuple(round(ca + (cb - ca) * t) for ca, cb in zip(a, b))


class Canvas:
    """A 32 x 32 tooltip; drawing stays inside its frame."""

    def __init__(self):
        self.img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
        self.px = self.img.load()
        for y in range(SIZE):
            for x in range(SIZE):
                if x in (0, SIZE - 1) and y in (0, SIZE - 1):
                    continue
                edge = x in (0, SIZE - 1) or y in (0, SIZE - 1)
                frame = x in (1, SIZE - 2) or y in (1, SIZE - 2)
                self.px[x, y] = lerp(FRAME_TOP, FRAME_BOTTOM, (y - 1) / (SIZE - 3)) if frame and not edge else BACKGROUND

    def set(self, x, y, c):
        if not (2 <= x < SIZE - 2 and 2 <= y < SIZE - 2) or c is None:
            return
        if len(c) == 4 and c[3] < 255:
            under, a = self.px[x, y], c[3] / 255
            c = tuple(round(u + (v - u) * a) for u, v in zip(under[:3], c[:3])) + (under[3],)
        self.px[x, y] = c if len(c) == 4 else c + (255,)

    def sprite(self, x0, y0, art, palette):
        for dy, row in enumerate(art):
            for dx, ch in enumerate(row):
                if ch not in ". ":
                    self.set(x0 + dx, y0 + dy, palette[ch])

    def save(self, path):
        self.img.resize((SIZE * SCALE, SIZE * SCALE), Image.NEAREST).save(path, optimize=True)


PALETTE = {
    # a slot: its fill, lit when the drag has crossed it, the shadow on its top left, the light on its bottom right
    "F": (0x8B, 0x8B, 0x8B), "f": (0xC4, 0xC4, 0xCC), "D": (0x37, 0x37, 0x3C), "B": (0xFF, 0xFF, 0xFF),
    # the item: an apple, its light, its shadow, its stalk and leaf
    "R": (0xD8, 0x30, 0x30), "r": (0xFF, 0x8C, 0x7C), "d": (0x8C, 0x18, 0x20), "s": (0x5C, 0x3C, 0x1C),
    "g": (0x4C, 0xB0, 0x38),
    # the cursor and its outline; the trail of the drag
    "W": (0xFF, 0xFF, 0xFF), "K": (0x10, 0x10, 0x14), "t": (0xFF, 0xFF, 0xFF, 0x90),
}

APPLE = [
    "..sg.",
    ".RsRR",
    "RrRRR",
    "RRRRd",
    ".RdR.",
]

CURSOR = [
    "K.......",
    "KK......",
    "KWK.....",
    "KWWK....",
    "KWWWK...",
    "KWWWWK..",
    "KWWWWWK.",
    "KWWWWWWK",
    "KWWWWKKK",
    "KWKKWK..",
    "KK.KWWK.",
    "....KWK.",
    ".....K..",
]


def slot(c, x0, y0, lit):
    """An inventory slot, 8 x 8 with its border, at (x0, y0)."""
    for y in range(8):
        for x in range(8):
            if (x == 0 or y == 0) and not (x == 7 or y == 7):
                colour = "D"
            elif (x == 7 or y == 7) and not (x == 0 or y == 0):
                colour = "B"
            else:
                colour = "f" if lit else "F"
            c.set(x0 + x, y0 + y, PALETTE[colour])


def draw():
    c = Canvas()
    # two rows of three slots; the drag has crossed the top row's first three
    for row in range(2):
        for col in range(3):
            lit = row == 0
            slot(c, 4 + col * 8, 11 + row * 8, lit)
            if lit:
                c.sprite(6 + col * 8, 13 + row * 8, APPLE, PALETTE)
    # where the drag started: what is left of the stack, in the bottom row
    c.sprite(6, 21, APPLE, PALETTE)
    # the trail the cursor has drawn, above the slots
    for x in range(7, 26):
        if x % 3 != 0:
            c.set(x, 7, PALETTE["t"])
    for x, y in [(5, 9), (6, 8), (25, 6), (25, 8), (26, 7)]:
        c.set(x, y, PALETTE["t"])
    c.sprite(21, 14, CURSOR, PALETTE)
    return c


if __name__ == "__main__":
    draw().save(sys.argv[1] if len(sys.argv) > 1 else "icon.png")
