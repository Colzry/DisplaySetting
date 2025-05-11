pub fn orientation_to_str(rotate: u32) -> &'static str {
    match rotate {
        0 => "横向",
        90 => "纵向",
        180 => "横向翻转",
        270 => "纵向翻转",
        _ => "未知方向",
    }
}
pub fn display_mode_to_str(mode: u32) -> &'static str {
    match mode {
        1 => "仅主屏",
        2 => "仅副屏",
        3 => "复制",
        4 => "扩展",
        _ => "未知模式",
    }
}