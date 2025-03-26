use clap::Parser;
use windows::Win32::Graphics::Gdi::*;
use windows::core::PCWSTR;
use windows::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_SETLOGICALDPIOVERRIDE, SPIF_UPDATEINIFILE, SPIF_SENDCHANGE};


#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// 刷新率 (Hz)，不指定自动获取最高的刷新率设置
    #[arg(short = 'r', long)]
    refresh_rate: Option<u32>,

    /// 显示方向 (0-横向/90-纵向/180-横向翻转/270-纵向翻转)，默认 0-横向
    #[arg(short = 'o', long, default_value_t = 0)]
    orientation: u32,

    /// 目标显示器 (1-显示器1/2-显示器2)，默认 1-显示器1
    #[arg(short = 'd', long, default_value_t = 1)]
    display: u32,

    /// 缩放比例 (100/125/150/175/200/225/250)，不传默认不修改
    #[arg(short = 's', long)]
    scaling: Option<u32>,

    /// 目标分辨率宽度，不传默认不修改
    #[arg(short = 'w', long)]
    width: Option<u32>,

    /// 目标分辨率高度，不传默认不修改
    #[arg(short = 'h', long)]
    height: Option<u32>,
}

fn main() {
    let args = Args::parse();

    if ![0, 90, 180, 270].contains(&args.orientation) {
        eprintln!("无效方向. 允许的值：0、90、180、270.");
        return;
    }

    if let Some(scaling) = args.scaling {
        if ![100, 125, 150, 175, 200, 225, 250].contains(&scaling) {
            eprintln!("无效缩放. 允许的值: 100, 125, 150, 175, 200, 225, 250.");
            return;
        }
    }

    if let Err(e) = change_display_settings(
        args.display,
        args.refresh_rate,
        args.orientation,
        args.scaling,
        args.width,
        args.height,
    ) {
        eprintln!("错误: {}", e);
    }
}

/// 自动获取最高刷新率
fn get_max_refresh_rate(device_name: PCWSTR) -> Option<u32> {
    let mut devmode = DEVMODEW {
        dmSize: std::mem::size_of::<DEVMODEW>() as u16,
        ..Default::default()
    };

    let mut max_refresh_rate = 0;
    let mut i = 0;

    while unsafe { EnumDisplaySettingsW(device_name, ENUM_DISPLAY_SETTINGS_MODE(i), &mut devmode) }.as_bool() {
        let current_rate = devmode.dmDisplayFrequency;
        if current_rate > max_refresh_rate {
            max_refresh_rate = current_rate;
        }
        i += 1;
    }

    if max_refresh_rate > 0 {
        Some(max_refresh_rate)
    } else {
        None
    }
}

/// 修改显示器的刷新率、方向、缩放比例和分辨率
fn change_display_settings(
    display_index: u32,
    refresh_rate: Option<u32>,
    orientation: u32,
    scaling: Option<u32>,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<(), String> {
    let mut device = DISPLAY_DEVICEW {
        cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
        ..Default::default()
    };

    if unsafe { EnumDisplayDevicesW(None, display_index - 1, &mut device, 0) }.as_bool() {
        let mut devmode = DEVMODEW {
            dmSize: std::mem::size_of::<DEVMODEW>() as u16,
            ..Default::default()
        };

        let device_name = PCWSTR(device.DeviceName.as_ptr());

        if unsafe { EnumDisplaySettingsW(device_name, ENUM_CURRENT_SETTINGS, &mut devmode) }.as_bool() {
            // 处理缩放比例 (优先处理，因为可能需要重启explorer)
            if let Some(scale) = scaling {
                // 使用系统API设置DPI缩放
                let dpi = match scale {
                    100 => 0, 125 => 1, 150 => 2,
                    175 => 3, 200 => 4, 225 => 5, 250 => 6,
                    _ => return Err("没有支持的缩放值".to_string()),
                };

                unsafe {
                    SystemParametersInfoW(
                        SPI_SETLOGICALDPIOVERRIDE,
                        dpi,
                        None,
                        SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
                    ).expect("无法应用缩放");
                }

                // 同时设置DEVMODE中的DPI值
                let log_pixels = match scale {
                    100 => 96, 125 => 120, 150 => 144,
                    175 => 168, 200 => 192, 225 => 216, 250 => 240,
                    _ => return Err("没有支持的缩放值".to_string()),
                };
                devmode.dmFields |= DM_LOGPIXELS;
                devmode.dmLogPixels = log_pixels as u16;
            }

            let target_refresh_rate = refresh_rate.unwrap_or_else(|| {
                get_max_refresh_rate(device_name).unwrap_or(devmode.dmDisplayFrequency)
            });

            let (new_width, new_height) = if width.is_some() && height.is_some() {
                (width.unwrap(), height.unwrap())
            } else {
                match orientation {
                    0 | 180 => (devmode.dmPelsWidth.max(devmode.dmPelsHeight), devmode.dmPelsWidth.min(devmode.dmPelsHeight)),
                    90 | 270 => (devmode.dmPelsWidth.min(devmode.dmPelsHeight), devmode.dmPelsWidth.max(devmode.dmPelsHeight)),
                    _ => return Err("没有支持的旋转角度".to_string()),
                }
            };

            devmode.dmFields |= DM_DISPLAYFREQUENCY | DM_DISPLAYORIENTATION |
                DM_PELSWIDTH | DM_PELSHEIGHT |
                DM_BITSPERPEL;
            devmode.dmPelsWidth = new_width;
            devmode.dmPelsHeight = new_height;
            devmode.dmDisplayFrequency = target_refresh_rate;
            devmode.dmBitsPerPel = 32;
            unsafe { devmode.Anonymous1.Anonymous2.dmDisplayFixedOutput = DEVMODE_DISPLAY_FIXED_OUTPUT(0); }

            unsafe {
                devmode.Anonymous1.Anonymous2.dmDisplayOrientation = match orientation {
                    0 => DMDO_DEFAULT,
                    90 => DMDO_90,
                    180 => DMDO_180,
                    270 => DMDO_270,
                    _ => return Err("没有支持的旋转角度".to_string()),
                };
            }

            // 测试模式
            let test_result = unsafe {
                ChangeDisplaySettingsExW(device_name, Some(&devmode as *const _), None, CDS_TEST, None)
            };

            if test_result != DISP_CHANGE_SUCCESSFUL {
                return Err("不支持所要求的显示设置".to_string());
            }

            // 应用设置 - 添加CDS_FULLSCREEN标志确保刷新率完全生效
            let result = unsafe {
                ChangeDisplaySettingsExW(
                    device_name,
                    Some(&devmode as *const _),
                    None,
                    CDS_GLOBAL | CDS_UPDATEREGISTRY | CDS_RESET | CDS_FULLSCREEN,
                    None,
                )
            };

            match result {
                DISP_CHANGE_SUCCESSFUL => Ok(()),
                DISP_CHANGE_BADMODE => Err("无效显示方式".to_string()),
                DISP_CHANGE_NOTUPDATED => Err("无法应用设置".to_string()),
                DISP_CHANGE_BADFLAGS => Err("无效标志".to_string()),
                DISP_CHANGE_FAILED => Err("显示驱动程序在指定的图形模式下失败".to_string()),
                _ => Err("未知错误".to_string()),
            }
        } else {
            Err("无法获取显示设置，请检查输入的参数".to_string())
        }
    } else {
        Err("找不到目标显示器，请检查输入的参数".to_string())
    }
}