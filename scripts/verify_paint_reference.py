"""Independent specimen-only pixel oracle; not Palette's HEIF implementation.
Run after the Rust paint_roundtrip example. Requires Pillow only for validation.
Known offsets are intentionally tied to the supplied SHA-256, not guessed files.
"""
import hashlib
from pathlib import Path
import sys
import zlib
from PIL import Image

root = Path(__file__).resolve().parent.parent
specimen = root / "crates/palette-core/tests/fixtures/microsoft-paint.paint"
data = specimen.read_bytes()
assert hashlib.sha256(data).hexdigest() == "b9b8547c9defb4b86182cf1bf2fc96f16d6e3abe2e220617fb5f3ec73d3a1bc9"
result = Image.new("RGBA", (819, 594), "white")
for offset, length in [(625, 3990), (4615, 5181)]:
    raw = zlib.decompress(data[offset:offset+length], -15)
    assert len(raw) == 819 * 594 * 4
    layer = Image.frombytes("RGBA", (819, 594), raw, "raw", "BGRA")
    result = Image.alpha_composite(result, layer)
expected = result.tobytes()
assert hashlib.sha256(expected).hexdigest() == "0bb07de787c44f5b48383408546ea42e979274b6d37dfdfbae7b46daba22e513"
output = Path(sys.argv[1]) if len(sys.argv) > 1 else root / "target/paint-roundtrip.png"
with Image.open(output) as image:
    assert image.size == (819, 594)
    assert image.convert("RGBA").tobytes() == expected
print("Independent specimen composite matches Palette PNG pixel for pixel.")
