//! 借用图像视图：判定层的纯数据输入。
//!
//! 捕获/渲染侧的图像类型（如 `golden_harness::capture::CapturedImage`）
//! 在各自所有者边界转换为本类型；判定层不反向依赖它们。

/// 紧凑 RGBA8 图像的借用视图（`width * height * 4` 字节）。
#[derive(Clone, Copy, Debug)]
pub struct ImageView<'a> {
    /// 宽（像素）。
    pub width: u32,
    /// 高（像素）。
    pub height: u32,
    /// 紧凑 RGBA8 像素（行主序，无行填充）。
    pub rgba8: &'a [u8],
}

impl<'a> ImageView<'a> {
    /// 构造并校验字节数与声明尺寸一致。
    #[must_use]
    pub fn new(width: u32, height: u32, rgba8: &'a [u8]) -> Option<Self> {
        (rgba8.len() == width as usize * height as usize * 4).then_some(Self {
            width,
            height,
            rgba8,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_construction_rejects_inconsistent_sizes() {
        let data = vec![0_u8; 32];
        assert!(ImageView::new(4, 2, &data).is_some());
        assert!(ImageView::new(2, 4, &data).is_some());
        assert!(ImageView::new(3, 3, &data).is_none());
        assert!(ImageView::new(4, 2, &data[..31]).is_none());
    }
}
