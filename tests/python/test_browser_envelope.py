from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'tools'))
from checks.browser import (
    ENVELOPE_MAX_DIFFERING_PIXELS,
    evaluate_browser_envelope,
)

W = H = 512


def flat_image(value=128):
    """Uniform mid-gray image (no edges anywhere)."""
    return bytes([value] * (W * H * 4))


def edge_image():
    """Left half dark / right half bright: a strong tier2 edge at x=256."""
    data = bytearray()
    for _y in range(H):
        for x in range(W):
            v = 40 if x < 256 else 220
            data.extend([v, v, v, 255])
    return bytes(data)


def with_pixel(image, x, y, delta):
    data = bytearray(image)
    o = (y * W + x) * 4
    data[o] = data[o] + delta
    return bytes(data)


class BrowserEnvelope(unittest.TestCase):
    def test_exact_pair_needs_no_envelope(self):
        image = flat_image()
        result = evaluate_browser_envelope(image, image)
        self.assertEqual(result['differing_pixels'], 0)
        self.assertTrue(result['edge_confined'])
        self.assertTrue(result['pass'])

    def test_small_edge_confined_divergence_passes(self):
        # 边缘窗口内的少量小差异（录制对的真实形态：6px / ≤31）→ envelope 成立。
        a = edge_image()
        b = a
        for x, y, delta in [(255, 100, 20), (258, 300, 31), (253, 200, 12)]:
            b = with_pixel(b, x, y, delta)
        result = evaluate_browser_envelope(a, b)
        self.assertEqual(result['differing_pixels'], 3)
        self.assertTrue(result['edge_confined'])
        self.assertTrue(result['pass'])

    def test_structural_non_edge_difference_fails(self):
        # 平坦区域的差异（结构性，非编译器舍入）→ 边缘约束失败。
        a = flat_image()
        b = with_pixel(a, 100, 100, 8)
        result = evaluate_browser_envelope(a, b)
        self.assertFalse(result['edge_confined'])
        self.assertFalse(result['pass'])

    def test_pixel_count_beyond_limit_fails(self):
        # 超过 13 个差异像素，即使全部贴边 → FAIL。
        a = edge_image()
        b = a
        for i in range(ENVELOPE_MAX_DIFFERING_PIXELS + 1):
            b = with_pixel(b, 255, 20 + i * 4, 10)
        result = evaluate_browser_envelope(a, b)
        self.assertEqual(result['differing_pixels'], ENVELOPE_MAX_DIFFERING_PIXELS + 1)
        self.assertTrue(result['edge_confined'])
        self.assertFalse(result['pass'])

    def test_channel_delta_beyond_limit_fails(self):
        # 单像素 delta 33/255（超过 32 上限），即使贴边 → FAIL。
        a = edge_image()
        b = with_pixel(a, 255, 100, 33)
        result = evaluate_browser_envelope(a, b)
        self.assertEqual(result['max_channel_delta'], 33)
        self.assertFalse(result['pass'])

    def test_edge_outside_window_fails(self):
        # 距 tier2 边缘 4px（超出 7×7 窗口）的差异 → 不算贴边。
        a = edge_image()
        b = with_pixel(a, 251, 100, 10)
        result = evaluate_browser_envelope(a, b)
        self.assertFalse(result['edge_confined'])
        self.assertFalse(result['pass'])


if __name__ == '__main__':
    unittest.main()
