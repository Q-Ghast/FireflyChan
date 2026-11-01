from collections import deque
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageOps

ROOT = Path(r"D:\Program\WinApp\FireflyChan")
SRC = Path(r"E:\Downloads\image.png")
ASSETS = ROOT / "src" / "assets"
RESOURCES = ROOT / "resources"
ASSETS.mkdir(parents=True, exist_ok=True)
RESOURCES.mkdir(parents=True, exist_ok=True)

img = Image.open(SRC).convert("RGB")
w, h = img.size
pixels = img.load()

# Flood fill the connected near-white background from the image border.
def is_bg(x: int, y: int) -> bool:
    r, g, b = pixels[x, y]
    return min(r, g, b) >= 190 and max(r, g, b) - min(r, g, b) <= 28

background = bytearray(w * h)
queue = deque()
for x in range(w):
    for y in (0, h - 1):
        if not background[y * w + x] and is_bg(x, y):
            background[y * w + x] = 1
            queue.append((x, y))
for y in range(h):
    for x in (0, w - 1):
        if not background[y * w + x] and is_bg(x, y):
            background[y * w + x] = 1
            queue.append((x, y))

while queue:
    x, y = queue.popleft()
    for nx, ny in ((x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)):
        if 0 <= nx < w and 0 <= ny < h and not background[ny * w + nx] and is_bg(nx, ny):
            background[ny * w + nx] = 1
            queue.append((nx, ny))

rgba = img.convert("RGBA")
rgba_pixels = rgba.load()
for y in range(h):
    row = y * w
    for x in range(w):
        if background[row + x]:
            r, g, b, _ = rgba_pixels[x, y]
            rgba_pixels[x, y] = (r, g, b, 0)

# Slightly soften the outer edge and remove the white JPEG fringe.
alpha = rgba.getchannel("A")
alpha = alpha.filter(ImageFilter.MinFilter(3))
alpha = alpha.filter(ImageFilter.GaussianBlur(0.6))
rgba.putalpha(alpha)

bbox = rgba.getbbox()
if bbox is None:
    raise SystemExit("background removal produced an empty image")
left, top, right, bottom = bbox
pad = 12
left = max(0, left - pad)
top = max(0, top - pad)
right = min(w, right + pad)
bottom = min(h, bottom + pad)
char = rgba.crop((left, top, right, bottom))

# Trim to a square-ish composition while keeping the full character.
side = max(char.width, char.height)
square = Image.new("RGBA", (side, side), (0, 0, 0, 0))
square.paste(char, ((side - char.width) // 2, side - char.height))

widget = square.resize((560, 560), Image.LANCZOS)
widget.save(ASSETS / "firefly-widget.png")
widget.save(RESOURCES / "firefly-widget.png")

# App icon: dark teal rounded square with the character and a soft glow.
size = 1024
icon = Image.new("RGBA", (size, size), (0, 0, 0, 0))
draw = ImageDraw.Draw(icon)
draw.rounded_rectangle((26, 26, size - 26, size - 26), radius=210, fill=(5, 20, 25, 255))
glow = Image.radial_gradient("L").resize((size, size))
glow = ImageOps.colorize(glow, (8, 42, 48), (5, 18, 23))
glow.putalpha(150)
icon.alpha_composite(glow)
draw.rounded_rectangle(
    (26, 26, size - 26, size - 26),
    radius=210,
    outline=(94, 234, 212, 150),
    width=6,
)

portrait = square.resize((860, 860), Image.LANCZOS)
portrait = portrait.crop((0, 70, 860, 930))
icon.alpha_composite(portrait, ((size - portrait.width) // 2, 118))
icon.save(RESOURCES / "app-icon.png")

print(f"source={w}x{h} bbox={bbox} widget={widget.size} icon={icon.size}")
