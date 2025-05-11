use clap::Parser;
#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Args {
    /// 刷新率 (Hz)，0 表示自动选择最大刷新率
    #[arg(short = 'r', long = "refresh-rate", value_name = "HZ")]
    pub refresh_rate: Option<u32>,

    /// 显示方向 (0-横向/90-纵向/180-横向翻转/270-纵向翻转)，不传默认不修改
    #[arg(short = 'o', long)]
    pub orientation: Option<u32>,

    /// 目标显示器 (1-显示器1/2-显示器2)，默认 1
    #[arg(short = 'd', long, default_value_t = 1)]
    pub display: u32,

    /// 目标分辨率宽度，需和高度一起设置，不传默认不修改
    #[arg(short = 'w', long)]
    pub width: Option<u32>,

    /// 目标分辨率高度，需和宽度一起设置，不传默认不修改
    #[arg(short = 'h', long)]
    pub height: Option<u32>,

    /// 显示模式 (1-仅主屏/2-仅副屏/3-复制/4-扩展)，不传默认不修改
    #[arg(short = 'm', long)]
    pub mode: Option<u32>,
}
