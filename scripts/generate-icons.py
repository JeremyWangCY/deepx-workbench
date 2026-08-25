from pathlib import Path
import sys

from PIL import Image

root = Path(__file__).resolve().parent.parent
source = Path(sys.argv[1]) if len(sys.argv) > 1 else root / "assets" / "deepx-icon-1024.png"
icons = root / "src-tauri" / "icons"
image = Image.open(source).convert("RGBA")

if image.size[0] != image.size[1]:
    raise SystemExit("The DeepX icon source must be square.")

icons.mkdir(parents=True, exist_ok=True)
assets = root / "assets"
assets.mkdir(parents=True, exist_ok=True)
image.save(assets / "deepx-icon.png", "PNG")

for name, size in {
    "32x32.png": 32,
    "64x64.png": 64,
    "128x128.png": 128,
    "128x128@2x.png": 256,
    "icon.png": 512,
    "Square30x30Logo.png": 30,
    "Square44x44Logo.png": 44,
    "Square71x71Logo.png": 71,
    "Square89x89Logo.png": 89,
    "Square107x107Logo.png": 107,
    "Square142x142Logo.png": 142,
    "Square150x150Logo.png": 150,
    "Square284x284Logo.png": 284,
    "Square310x310Logo.png": 310,
    "StoreLogo.png": 50,
}.items():
    image.resize((size, size), Image.Resampling.LANCZOS).save(icons / name, "PNG")

image.save(
    icons / "icon.ico",
    "ICO",
    sizes=[(16, 16), (20, 20), (24, 24), (32, 32), (40, 40), (48, 48), (64, 64), (128, 128), (256, 256)],
)
image.save(icons / "icon.icns", "ICNS")

for platform_icon in list((icons / "android").rglob("*.png")) + list((icons / "ios").glob("*.png")):
    with Image.open(platform_icon) as existing:
        size = existing.size
    image.resize(size, Image.Resampling.LANCZOS).save(platform_icon, "PNG")