"""Generate brand assets from logo-source.jpg.

Outputs (apps/zylcode-desktop/branding/):
  icon-source.png  1024x1024 emblem tile for `tauri icon` (bg #141414)
  wordmark.png     trimmed ZYLCODE wordmark on #141414 for the app header
  lockup.png       full emblem+wordmark on #141414 (README/docs)
  favicon.png      64x64 favicon (copied to public/)
"""
from PIL import Image
import numpy as np

SRC = "apps/zylcode-desktop/branding/logo-source.jpg"
BG = (20, 20, 20)  # exact background so JPEG seams disappear on the chip

im = Image.open(SRC).convert("RGB")
a = np.asarray(im).astype(int)
lum = a.mean(axis=2)

# --- Emblem tile (square, centered, generous margins for the crosshair) ----
ex0, ex1, ey0, ey1 = 37, 360, 78, 336
emblem = im.crop((ex0, ey0, ex1, ey1))
side = 1024
tile = Image.new("RGB", (side, side), BG)
# Fit emblem into ~86% of the tile, centered slightly toward visual balance.
target = int(side * 0.86)
scale = min(target / emblem.width, target / emblem.height)
ew, eh = round(emblem.width * scale), round(emblem.height * scale)
emblem_resized = emblem.resize((ew, eh), Image.LANCZOS)
# Flatten JPEG noise onto the exact BG color: composite via alpha from diff.
ea = np.asarray(emblem_resized).astype(int)
diff = np.abs(ea - np.array(BG)).sum(axis=2)
alpha = np.clip((diff - 6) * 18, 0, 255).astype(np.uint8)  # bg -> transparent
emblem_rgba = np.dstack([ea.astype(np.uint8), alpha])
tile_rgba = Image.new("RGBA", (side, side), BG + (255,))
tile_rgba.paste(Image.fromarray(emblem_rgba, "RGBA"),
                ((side - ew) // 2, (side - eh) // 2), Image.fromarray(emblem_rgba, "RGBA"))
tile_rgba.convert("RGB").save("apps/zylcode-desktop/branding/icon-source.png")

# --- Wordmark (trimmed, flattened to exact BG for the header chip) ---------
wx0, wx1, wy0, wy1 = 352, 1094, 172, 288
word = im.crop((wx0, wy0, wx1, wy1))
wa = np.asarray(word).astype(int)
wdiff = np.abs(wa - np.array(BG)).sum(axis=2)
wmask = wdiff > 10
ys, xs = np.where(wmask)
word = word.crop((xs.min(), ys.min(), xs.max() + 1, ys.max() + 1))
out = Image.new("RGB", word.size, BG)
walpha = np.clip((np.abs(np.asarray(word).astype(int) - np.array(BG)).sum(axis=2) - 6) * 18, 0, 255).astype(np.uint8)
out.paste(word, (0, 0), Image.fromarray(np.dstack([np.asarray(word).astype(np.uint8), walpha]), "RGBA"))
out.save("apps/zylcode-desktop/branding/wordmark.png")
print("wordmark:", out.size)

# --- Full lockup (README / docs) -------------------------------------------
lock = im.copy()
la = np.asarray(lock).astype(int)
lalpha = np.clip((np.abs(la - np.array(BG)).sum(axis=2) - 6) * 18, 0, 255).astype(np.uint8)
lock_rgba = Image.new("RGBA", lock.size, BG + (0,))
lock_rgba.paste(lock, (0, 0), Image.fromarray(np.dstack([la.astype(np.uint8), lalpha]), "RGBA"))
flat = Image.new("RGB", lock.size, BG)
flat.paste(lock_rgba, (0, 0), lock_rgba)
flat.save("apps/zylcode-desktop/branding/lockup.png")

# --- Favicon ---------------------------------------------------------------
tile_rgba.resize((64, 64), Image.LANCZOS).save("apps/zylcode-desktop/public/favicon.png")
print("done")
