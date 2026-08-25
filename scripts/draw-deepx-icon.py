from pathlib import Path

import cv2
import numpy as np

SIZE = 1024
INK = (11, 11, 12, 255)
WARM_WHITE = (245, 243, 238, 255)
OUTPUT = Path(__file__).resolve().parent.parent / "assets" / "deepx-icon-1024.png"


def cubic(start, control_one, control_two, end, steps=32):
    points = []
    for value in np.linspace(0, 1, steps, endpoint=False):
        points.append(
            (1 - value) ** 3 * np.array(start)
            + 3 * (1 - value) ** 2 * value * np.array(control_one)
            + 3 * (1 - value) * value**2 * np.array(control_two)
            + value**3 * np.array(end)
        )
    return points


def rounded_square(image, color, inset, radius):
    corner = inset + radius
    cv2.rectangle(image, (corner, inset), (SIZE - corner, SIZE - inset), color, -1)
    cv2.rectangle(image, (inset, corner), (SIZE - inset, SIZE - corner), color, -1)
    for x, y in ((corner, corner), (SIZE - corner, corner), (corner, SIZE - corner), (SIZE - corner, SIZE - corner)):
        cv2.circle(image, (x, y), radius, color, -1, cv2.LINE_AA)


def curve_points(segments):
    points = []
    for segment in segments:
        points.extend(cubic(*segment))
    return np.rint(np.array(points)).astype(np.int32)


image = np.zeros((SIZE, SIZE, 4), dtype=np.uint8)
rounded_square(image, INK, inset=20, radius=220)

tail = curve_points(
    [
        ((512, 777), (486, 769), (466, 754), (463, 733)),
        ((463, 733), (454, 659), (476, 615), (407, 576)),
        ((407, 576), (278, 505), (158, 468), (147, 326)),
        ((147, 326), (144, 284), (151, 259), (169, 247)),
        ((169, 247), (192, 278), (264, 311), (366, 327)),
        ((366, 327), (435, 337), (487, 365), (512, 428)),
        ((512, 428), (537, 365), (589, 337), (658, 327)),
        ((658, 327), (760, 311), (832, 278), (855, 247)),
        ((855, 247), (873, 259), (880, 284), (877, 326)),
        ((877, 326), (866, 468), (746, 505), (617, 576)),
        ((617, 576), (548, 615), (570, 659), (561, 733)),
        ((561, 733), (558, 754), (538, 769), (512, 777)),
    ]
)
cv2.fillPoly(image, [tail], WARM_WHITE, cv2.LINE_AA)

scar = np.zeros((SIZE, SIZE), dtype=np.uint8)
for start, end in (((470, 592), (550, 672)), ((554, 592), (474, 672))):
    cv2.line(scar, start, end, 255, 15, cv2.LINE_AA)
    cv2.circle(scar, start, 7, 255, -1, cv2.LINE_AA)
    cv2.circle(scar, end, 7, 255, -1, cv2.LINE_AA)
image[scar > 0] = INK

OUTPUT.parent.mkdir(parents=True, exist_ok=True)
cv2.imwrite(str(OUTPUT), image)
