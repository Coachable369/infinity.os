"""Coordinate conversion for native framebuffer hit regions and normalized input."""


# ------------------------=
# FUNC: title_target
# DESC: Converts an actual Settings title-bar pixel center to the normalized coordinates reported by the native pointer.
# ------------------=
def title_target(rect, width, height):
    x, y, w, h = rect
    scale = 2 if width >= 2560 and height >= 1440 else 1
    assert width > 0 and height > 0
    assert x >= 0 and y >= 0 and w > 0 and h >= 54 * scale
    assert x + w <= width and y + h <= height
    pixel_x, pixel_y = x + w // 2, y + 20 * scale
    return pixel_x * 1000 // width, pixel_y * 1000 // height
