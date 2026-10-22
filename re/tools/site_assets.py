"""Make the images of the website (docs/site/) from the data pack and from
screens the game exports.

    ouro-scout --export 68 shots/l68.bmp
    ouro-scout --export 17 shots/levelscreen.bmp level
    ouro-scout --export-menu menu shots/menu.bmp
    (convert the BMPs to PNG, e.g. with sips)
    python3 site_assets.py data/ouro-scout-data.zip shots docs/site

Writes: title.png (the title picture as the original shows it, 512 x 364),
about.png, box.png (the 3 x 3 marble tiles for CSS border-image),
screen-*.png (game screens, 2x), heading-*.png (block letters) and
player.png. Needs Pillow.
"""
import io
import os
import sys
import zipfile

from PIL import Image

# Headings in the game's block letters (file name: text).
HEADINGS = {
    "logo": "OURO S.C.OUT",
    "game": "THE GAME",
    "faithful": "AS IN 1993",
    "screens": "SCREENS",
    "play": "PLAY",
    "keys": "KEYS",
    "credits": "CREDITS",
}
# Frame area of the 640 x 480 screen (where the original draws pictures).
FRAME = (64, 68, 64 + 512, 68 + 364)


def glyph(c):
    if "A" <= c <= "Z":
        return ord(c) - ord("A")
    if "1" <= c <= "9":
        return ord(c) - ord("1") + 26
    return {"0": 35, "!": 36, "?": 37, ".": 38, ",": 39, ":": 40, ";": 41, "=": 42, "+": 43, "-": 44}.get(c)


def main():
    pack, shots, out = sys.argv[1:4]
    os.makedirs(out, exist_ok=True)
    z = zipfile.ZipFile(pack)

    def image(name):
        return Image.open(io.BytesIO(z.read(name))).convert("RGBA")

    # the title picture, clipped like the original (CODE 8 0x12d8)
    image("pictures/1000.png").crop((0, 0, 512, 364)).save(f"{out}/title.png", optimize=True)
    image("pictures/256.png").save(f"{out}/about.png", optimize=True)
    box = Image.new("RGBA", (96, 96))
    for i in range(9):
        box.paste(image(f"graphics/box/{i}.png"), ((i % 3) * 32, (i // 3) * 32))
    box.save(f"{out}/box.png", optimize=True)
    image("graphics/objects/15.png").save(f"{out}/player.png", optimize=True)

    font = [image(f"graphics/font/{i:02d}.png") for i in range(45)]
    for name, text in HEADINGS.items():
        # the original sets letters 32 pixels apart; spaces narrower here
        widths = [20 if c == " " else 32 for c in text]
        img = Image.new("RGBA", (sum(widths), 32))
        x = 0
        for c, w in zip(text, widths):
            g = glyph(c)
            if g is not None:
                img.alpha_composite(font[g], (x, 0))
            x += w
        img.save(f"{out}/heading-{name}.png", optimize=True)

    def screen(src, dst, scale_from=1):
        img = Image.open(f"{shots}/{src}").convert("RGB")
        box = tuple(v * scale_from for v in FRAME)
        img = img.crop(box)
        img.resize((1024, 728), Image.NEAREST).save(f"{out}/{dst}", optimize=True)

    screen("l68.png", "screen-level68.png")
    screen("levelscreen.png", "screen-levelscreen.png")
    screen("menu.png", "screen-menu.png", scale_from=2)
    print("written to", out)


if __name__ == "__main__":
    main()
