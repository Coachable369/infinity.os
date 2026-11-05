"""figlet [-f standard|small] [-o FILE] TEXT -- full-width FIGfont rendering."""
import argparse

p = argparse.ArgumentParser(description=__doc__)
p.add_argument("-f", "--font", choices=("standard", "small"), default="standard")
p.add_argument("-o", "--output")
p.add_argument("text", nargs="+")
a = p.parse_args()
with open("/System/" + a.font + ".flf", encoding="latin-1") as stream:
    header = stream.readline().split()
    if not header[0].startswith("flf2a"):
        raise ValueError("Not a FIGfont")
    hardblank, height = header[0][-1], int(header[1])
    for _ in range(int(header[5])):
        stream.readline()
    glyphs = {}
    for code in range(32, 127):
        rows = [stream.readline().rstrip("\r\n") for _ in range(height)]
        mark = rows[0][-1]
        glyphs[chr(code)] = [r.rstrip(mark).replace(hardblank, " ") for r in rows]
banner = "\n".join("".join(glyphs.get(c, glyphs["?"])[i] for c in " ".join(a.text)).rstrip()
                   for i in range(height)) + "\n"
if a.output:
    with open(a.output, "w", encoding="utf-8") as stream:
        stream.write(banner)
else:
    print(banner, end="")
