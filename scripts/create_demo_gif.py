#!/usr/bin/env python3
"""Generate an animated GIF demo of gopher-cli's terminal UI.

Renders a pixel-perfect simulation of the ratatui two-pane TUI,
showcasing Hacker News RSS browsing and Gopherspace navigation.
"""

from PIL import Image, ImageDraw, ImageFont
import os, sys

# ── Configuration ────────────────────────────────────────────────

COLS = 96
ROWS = 28
FONT_PATH = "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"
FONT_SIZE = 14
CHAR_W = 9       # measured advance width
CHAR_H = 19      # line height (ascent + descent + spacing)
PADDING = 12
TITLEBAR_H = 30  # macOS-style window chrome
OUTPUT = os.path.join(os.path.dirname(os.path.dirname(__file__)), "demo.gif")

# Menu/Content split (40/60)
MENU_W = 38
CONTENT_W = COLS - MENU_W  # 58
BODY_H = ROWS - 1          # rows for panes (last row = status bar)

# ── Colors ───────────────────────────────────────────────────────

C = {
    "bg":        (13,  17,  23),
    "fg":        (201, 209, 217),
    "cyan":      (86,  209, 219),
    "yellow":    (229, 192, 123),
    "green":     (152, 195, 121),
    "magenta":   (198, 120, 221),
    "darkgray":  (110, 118, 129),
    "white":     (224, 228, 233),
    "red":       (240, 113, 120),
    "black":     (1,   4,   9),
    "sel_bg":    (48,  54,  61),
    "status_bg": (22,  27,  34),
    "green_sel": (30,  70,  35),
    "popup_bg":  (22,  27,  34),
    "titlebar":  (30,  34,  42),
    "titletxt":  (139, 148, 158),
}

# ── Cell / Screen ────────────────────────────────────────────────

class Cell:
    __slots__ = ("ch", "fg", "bg")
    def __init__(self, ch=" ", fg="fg", bg="bg"):
        self.ch = ch; self.fg = fg; self.bg = bg

class Screen:
    def __init__(self):
        self.grid = [[Cell() for _ in range(COLS)] for _ in range(ROWS)]

    def clear(self):
        for r in range(ROWS):
            for c in range(COLS):
                self.grid[r][c] = Cell()

    def put(self, r, c, ch, fg="fg", bg="bg"):
        if 0 <= r < ROWS and 0 <= c < COLS:
            self.grid[r][c] = Cell(ch, fg, bg)

    def text(self, r, c, s, fg="fg", bg="bg"):
        for i, ch in enumerate(s):
            self.put(r, c + i, ch, fg, bg)

    def hline(self, r, c, w, fg="fg"):
        for i in range(w): self.put(r, c + i, "\u2500", fg)

    def box(self, top, left, h, w, color="cyan", title=None):
        self.put(top, left, "\u250c", color)
        self.put(top, left + w - 1, "\u2510", color)
        self.put(top + h - 1, left, "\u2514", color)
        self.put(top + h - 1, left + w - 1, "\u2518", color)
        self.hline(top, left + 1, w - 2, color)
        self.hline(top + h - 1, left + 1, w - 2, color)
        for r in range(top + 1, top + h - 1):
            self.put(r, left, "\u2502", color)
            self.put(r, left + w - 1, "\u2502", color)
        if title:
            self.text(top, left + 1, title, color)

    def fill_bg(self, r, c0, c1, bg):
        for c in range(c0, c1):
            if 0 <= r < ROWS and 0 <= c < COLS:
                self.grid[r][c].bg = bg

    def fill_area(self, top, left, h, w, bg):
        for r in range(top, top + h):
            self.fill_bg(r, left, left + w, bg)

    def render(self, font):
        cw, ch = CHAR_W, CHAR_H
        pad = PADDING
        img_w = COLS * cw + pad * 2
        img_h = ROWS * ch + pad * 2 + TITLEBAR_H
        img = Image.new("RGB", (img_w, img_h), C["bg"])
        draw = ImageDraw.Draw(img)

        # ── Window chrome ──
        draw.rectangle([0, 0, img_w, TITLEBAR_H], fill=C["titlebar"])
        # Traffic-light dots
        dot_y = TITLEBAR_H // 2
        for i, color in enumerate([(255, 95, 86), (255, 189, 46), (39, 201, 63)]):
            draw.ellipse(
                [pad + i * 22 - 6, dot_y - 6, pad + i * 22 + 6, dot_y + 6],
                fill=color,
            )
        # Title text
        title = "gopher-cli"
        tw = int(font.getlength(title))
        draw.text(((img_w - tw) // 2, dot_y - 7), title, fill=C["titletxt"], font=font)

        # ── Terminal cells ──
        y_off = TITLEBAR_H + pad
        for r in range(ROWS):
            for c in range(COLS):
                cell = self.grid[r][c]
                x = pad + c * cw
                y = y_off + r * ch
                if cell.bg != "bg":
                    draw.rectangle([x, y, x + cw - 1, y + ch - 1], fill=C[cell.bg])
                if cell.ch != " ":
                    draw.text((x, y + 1), cell.ch, fill=C[cell.fg], font=font)
        return img

# ── Layout Drawing ───────────────────────────────────────────────

def _type_props(t):
    """Return (indicator, color) for a Gopher item type."""
    return {
        "1": ("[+]", "yellow"),
        "0": ("[T]", "white"),
        "7": ("[?]", "green"),
        "h": ("[H]", "magenta"),
        "i": ("   ", "darkgray"),
    }.get(t, ("[.]", "darkgray"))


def draw_layout(scr, *, path="", items=(), sel=0, content=(),
                menu_focus=True, loading=False):
    """Render the standard two-pane TUI layout."""
    scr.clear()
    mb = "cyan" if menu_focus else "darkgray"
    cb = "darkgray" if menu_focus else "cyan"

    # Menu box
    mtitle = f" >(^.^)> {path} " if path else " >(^.^)> / "
    scr.box(0, 0, BODY_H, MENU_W, mb, mtitle)

    # Content box
    scr.box(0, MENU_W, BODY_H, CONTENT_W, cb, " Content ")

    # ── Menu items ──
    max_label = MENU_W - 2 - 7  # border(2) + ">> [+] "(7)
    for i, (itype, label) in enumerate(items):
        r = 1 + i
        if r >= BODY_H - 1:
            break
        ind, color = _type_props(itype)
        lbl = label[:max_label]
        if i == sel:
            scr.fill_bg(r, 1, MENU_W - 1, "sel_bg")
            scr.text(r, 1, ">> ", color, "sel_bg")
            scr.text(r, 4, f"{ind} {lbl}", color, "sel_bg")
        else:
            scr.text(r, 4, f"{ind} {lbl}", color)

    # ── Content lines ──
    max_text = CONTENT_W - 3  # border(2) + 1 padding
    for i, (line, color) in enumerate(content):
        r = 1 + i
        if r >= BODY_H - 1:
            break
        scr.text(r, MENU_W + 2, line[:max_text], color)

    # ── Status bar ──
    scr.fill_bg(ROWS - 1, 0, COLS, "status_bg")
    p = path or "/"
    scr.text(ROWS - 1, 1, p, "cyan", "status_bg")
    col = len(p) + 2
    if loading:
        scr.text(ROWS - 1, col, " loading\u2026 ", "yellow", "status_bg")
        col += 11
    help_str = " q:quit  b:back  /:search  ::goto  Tab:pane  Enter:open"
    scr.text(ROWS - 1, col, help_str, "darkgray", "status_bg")


def draw_goto(scr, query, items, gsel):
    """Overlay the GoTo popup on the current screen."""
    pw = max(int(COLS * 0.58), 30)
    ph = max(int(ROWS * 0.60), 8)
    x0 = (COLS - pw) // 2
    y0 = (ROWS - ph) // 2

    # Clear + background
    scr.fill_area(y0, x0, ph, pw, "popup_bg")
    for r in range(y0, y0 + ph):
        for c in range(x0, x0 + pw):
            if scr.grid[r][c].ch != " ":
                scr.grid[r][c] = Cell(" ", "fg", "popup_bg")

    # Input box (3 rows tall)
    cnt = len(items)
    scr.box(y0, x0, 3, pw, "green", f" Go to ({cnt}) Tab:expand ")
    # Clear inside input box
    for c in range(x0 + 1, x0 + pw - 1):
        scr.put(y0 + 1, c, " ", "fg", "popup_bg")
    scr.text(y0 + 1, x0 + 1, "> ", "green", "popup_bg")
    scr.text(y0 + 1, x0 + 3, query, "fg", "popup_bg")
    scr.text(y0 + 1, x0 + 3 + len(query), "_", "green", "popup_bg")

    # List area borders (no top, continues from input box)
    list_top = y0 + 3
    bottom = y0 + ph - 1
    for r in range(list_top, bottom):
        scr.put(r, x0, "\u2502", "green")
        scr.put(r, x0 + pw - 1, "\u2502", "green")
    scr.put(bottom, x0, "\u2514", "green")
    scr.put(bottom, x0 + pw - 1, "\u2518", "green")
    scr.hline(bottom, x0 + 1, pw - 2, "green")

    # Clear list interior
    for r in range(list_top, bottom):
        for c in range(x0 + 1, x0 + pw - 1):
            scr.put(r, c, " ", "fg", "popup_bg")

    # Items
    max_w = pw - 5
    for i, (depth, is_dir, expanded, label) in enumerate(items):
        r = list_top + i
        if r >= bottom:
            break
        indent = "  " * depth
        marker = ("[-] " if expanded else "[+] ") if is_dir else "    "
        color = "yellow" if is_dir else "white"
        txt = f"{indent}{marker}{label}"[:max_w]
        if i == gsel:
            scr.fill_bg(r, x0 + 1, x0 + pw - 1, "green_sel")
            scr.text(r, x0 + 2, "> ", "green", "green_sel")
            scr.text(r, x0 + 4, txt, color, "green_sel")
        else:
            scr.text(r, x0 + 4, txt, color, "popup_bg")

# ── Scene Data ───────────────────────────────────────────────────

ROOT_ITEMS = [
    ("1", "local"),
    ("1", "feed.hackernews"),
    ("1", "gopher.floodgap.com"),
    ("1", "gopher.quux.org"),
    ("1", "gopherpedia.com"),
    ("1", "cosmic.voyage"),
    ("1", "sdf.org"),
    ("1", "bitreich.org"),
]

WELCOME = [
    ("", "fg"),
    ("       >(^.^)>", "cyan"),
    ("", "fg"),
    ("      gopher-cli", "white"),
    ("", "fg"),
    (" Select an item to view", "darkgray"),
    (" its content.", "darkgray"),
    ("", "fg"),
    (" Unified browser for gopherspace,", "darkgray"),
    (" RSS feeds, files & knowledge", "darkgray"),
    (" graphs.", "darkgray"),
]

HN_ITEMS = [
    ("0", "Show HN: Gopher-CLI \u2013 terminal browser"),
    ("0", "Rust 2025 edition is now stable"),
    ("0", "The small web is beautiful"),
    ("0", "Ask HN: Favorite TUI applications?"),
    ("0", "SQLite 4.0 design notes released"),
    ("0", "Deep dive into terminal emulators"),
    ("0", "Why I still use Gopher in 2026"),
    ("0", "Structured content discovery tools"),
    ("0", "Building CLI tools in Rust"),
    ("0", "SPARQL for beginners"),
    ("0", "A love letter to plain text"),
    ("0", "Gemini protocol one year later"),
]

HN_ARTICLE = [
    ("Show HN: Gopher-CLI", "white"),
    ("A terminal browser for gopherspace", "white"),
    ("\u2500" * 40, "darkgray"),
    ("342 points | 127 comments | 2h ago", "darkgray"),
    ("by gopherfan", "darkgray"),
    ("", "fg"),
    ("I built a unified terminal browser", "fg"),
    ("that combines Gopherspace, RSS feeds,", "fg"),
    ("local files, and RDF knowledge graphs", "fg"),
    ("into a single navigable interface.", "fg"),
    ("", "fg"),
    ("Everything uses Gopher\u2019s simple menu/", "fg"),
    ("document model, so you browse all", "fg"),
    ("sources with the same keystrokes.", "fg"),
    ("", "fg"),
    ("Features:", "yellow"),
    (" \u2022 Interactive TUI with two-pane view", "fg"),
    (" \u2022 CLI with auto-JSON pipe output", "fg"),
    (" \u2022 RSS/Atom feed integration", "fg"),
    (" \u2022 Live Gopher server browsing", "fg"),
    (" \u2022 RDF/SPARQL knowledge graphs", "fg"),
    (" \u2022 File system vault for notes", "fg"),
    ("", "fg"),
    ("github.com/user/gopher-cli", "cyan"),
]

GOPHER_ITEMS = [
    ("i", "Welcome to Floodgap Systems"),
    ("i", "Official Gopher Server"),
    ("i", ""),
    ("1", "About This Server"),
    ("1", "Fun and Games"),
    ("1", "Gopher Project"),
    ("1", "Technical Information"),
    ("1", "Weather Maps & Data"),
    ("1", "The Overbite Project"),
    ("0", "What is Gopher?"),
    ("7", "Search Gopherspace (Veronica-2)"),
    ("i", ""),
    ("i", "Last updated: 2026-03-01"),
]

GOPHER_CONTENT = [
    ("Welcome to Floodgap Systems", "white"),
    ("\u2550" * 35, "darkgray"),
    ("", "fg"),
    ("This is the Floodgap gopher server.", "fg"),
    ("We\u2019ve been serving the Gopherverse", "fg"),
    ("since 1999.", "fg"),
    ("", "fg"),
    ("The Gopher protocol is a simple,", "fg"),
    ("text-based information retrieval", "fg"),
    ("protocol that predates the World", "fg"),
    ("Wide Web.", "fg"),
    ("", "fg"),
    ("Despite its age, Gopher remains a", "fg"),
    ("vibrant space for those who value", "fg"),
    ("simplicity and text-based content.", "fg"),
    ("", "fg"),
    ("Explore our menus to discover fun", "fg"),
    ("content, technical info, or search", "fg"),
    ("the wider Gopherspace.", "fg"),
    ("", "fg"),
    ('   "In a world of complexity,', "yellow"),
    ('    Gopher keeps it simple."', "yellow"),
    ("", "fg"),
    ("Server uptime: 9847 days", "darkgray"),
]

GOTO_ALL = [
    (0, True,  False, "local"),
    (0, True,  False, "feed.hackernews"),
    (0, True,  False, "gopher.floodgap.com"),
    (0, True,  False, "gopher.quux.org"),
    (0, True,  False, "gopherpedia.com"),
    (0, True,  False, "cosmic.voyage"),
    (0, True,  False, "sdf.org"),
    (0, True,  False, "bitreich.org"),
]

def goto_filter(q):
    if not q:
        return GOTO_ALL[:]
    return [it for it in GOTO_ALL if q.lower() in it[3].lower()]

# ── Frame Sequence ───────────────────────────────────────────────

def build_frames(font):
    """Return list of (PIL.Image, duration_ms)."""
    scr = Screen()
    frames = []

    def snap(ms):
        frames.append((scr.render(font), ms))

    # 1 ── Root menu, welcome
    draw_layout(scr, items=ROOT_ITEMS, sel=0, content=WELCOME)
    snap(2200)

    # 2 ── Cursor to feed.hackernews
    draw_layout(scr, items=ROOT_ITEMS, sel=1, content=WELCOME)
    snap(800)

    # 3 ── Enter HN: loading
    draw_layout(scr, path="feed.hackernews/", items=[], sel=0,
                content=[], loading=True)
    snap(550)

    # 4 ── HN entries loaded
    draw_layout(scr, path="feed.hackernews/", items=HN_ITEMS, sel=0,
                content=WELCOME)
    snap(1200)

    # 5-7 ── Browse entries
    for s in (1, 2, 3):
        draw_layout(scr, path="feed.hackernews/", items=HN_ITEMS, sel=s,
                    content=WELCOME)
        snap(320)

    # 8 ── Back to first entry
    draw_layout(scr, path="feed.hackernews/", items=HN_ITEMS, sel=0,
                content=WELCOME)
    snap(400)

    # 9 ── Read article (content pane focus)
    draw_layout(scr, path="feed.hackernews/", items=HN_ITEMS, sel=0,
                content=HN_ARTICLE, menu_focus=False)
    snap(3500)

    # 10 ── Back to root
    draw_layout(scr, items=ROOT_ITEMS, sel=1, content=WELCOME)
    snap(500)

    # 11 ── Cursor to gopher.floodgap.com
    draw_layout(scr, items=ROOT_ITEMS, sel=2, content=WELCOME)
    snap(800)

    # 12 ── Enter gopher: loading
    draw_layout(scr, path="gopher.floodgap.com/", items=[], sel=0,
                content=[], loading=True)
    snap(650)

    # 13 ── Gopher menu
    draw_layout(scr, path="gopher.floodgap.com/", items=GOPHER_ITEMS, sel=0,
                content=WELCOME)
    snap(1000)

    # 14-16 ── Navigate to "About This Server" (index 3)
    for s in (1, 2, 3):
        draw_layout(scr, path="gopher.floodgap.com/", items=GOPHER_ITEMS,
                    sel=s, content=WELCOME)
        snap(280)

    # 17 ── Read gopher content
    draw_layout(scr, path="gopher.floodgap.com/", items=GOPHER_ITEMS, sel=3,
                content=GOPHER_CONTENT, menu_focus=False)
    snap(3500)

    # 18 ── Open GoTo popup (all items)
    draw_layout(scr, path="gopher.floodgap.com/", items=GOPHER_ITEMS, sel=3,
                content=GOPHER_CONTENT)
    draw_goto(scr, "", GOTO_ALL, 0)
    snap(1100)

    # 19-22 ── Type "goph" character by character
    for q in ("g", "go", "gop", "goph"):
        draw_layout(scr, path="gopher.floodgap.com/", items=GOPHER_ITEMS,
                    sel=3, content=GOPHER_CONTENT)
        filtered = goto_filter(q)
        draw_goto(scr, q, filtered, 0)
        snap(320)

    # 23 ── Filtered result
    draw_layout(scr, path="gopher.floodgap.com/", items=GOPHER_ITEMS,
                sel=3, content=GOPHER_CONTENT)
    draw_goto(scr, "goph", goto_filter("goph"), 0)
    snap(1500)

    # 24 ── Back to root (final)
    draw_layout(scr, items=ROOT_ITEMS, sel=0, content=WELCOME)
    snap(2500)

    return frames

# ── Main ─────────────────────────────────────────────────────────

def main():
    font = ImageFont.truetype(FONT_PATH, FONT_SIZE)
    print(f"Terminal: {COLS}\u00d7{ROWS}  |  Char: {CHAR_W}\u00d7{CHAR_H}  |  Font: {FONT_SIZE}pt")

    img_w = COLS * CHAR_W + PADDING * 2
    img_h = ROWS * CHAR_H + PADDING * 2 + TITLEBAR_H
    print(f"Image: {img_w}\u00d7{img_h}")

    frames = build_frames(font)
    print(f"Frames: {len(frames)}")

    images   = [f[0] for f in frames]
    durations = [f[1] for f in frames]

    # Quantize to 64-color palette for smaller file
    images_p = [im.quantize(colors=64, method=Image.Quantize.MEDIANCUT)
                for im in images]

    images_p[0].save(
        OUTPUT,
        save_all=True,
        append_images=images_p[1:],
        duration=durations,
        loop=0,
        optimize=True,
    )

    size_kb = os.path.getsize(OUTPUT) / 1024
    total_ms = sum(durations)
    print(f"Saved: {OUTPUT}  ({size_kb:.0f} KB, {total_ms/1000:.1f}s, {len(frames)} frames)")

if __name__ == "__main__":
    main()
