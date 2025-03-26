use clap::Parser;
use windows::Win32::Graphics::Gdi::*;
use windows::core::PCWSTR;

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
}

fn main() {
    let args = Args::parse();

    if ![0, 90, 180, 270].contains(&args.orientation) {
        eprintln!("无效方向. 允许的值：0、90、180、270.");
        return;
    }

    if let Err(e) = change_display_settings(args.display, args.refresh_rate, args.orientation) {
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

/// 修改显示器刷新率和方向
fn change_display_settings(display_index: u32, refresh_rate: Option<u32>, orientation: u32) -> Result<(), String> {
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
            let target_refresh_rate = refresh_rate.unwrap_or_else(|| {
                get_max_refresh_rate(device_name).unwrap_or(60) // 默认 60Hz
            });

            let (new_width, new_height) = match orientation {
                0 | 180 => (devmode.dmPelsWidth.max(devmode.dmPelsHeight), devmode.dmPelsWidth.min(devmode.dmPelsHeight)),
                90 | 270 => (devmode.dmPelsWidth.min(devmode.dmPelsHeight), devmode.dmPelsWidth.max(devmode.dmPelsHeight)),
                _ => return Err("没有支持的旋转角度".to_string()),
            };

            devmode.dmFields = DM_DISPLAYFREQUENCY | DM_DISPLAYORIENTATION | DM_PELSWIDTH | DM_PELSHEIGHT | DM_BITSPERPEL;
            devmode.dmPelsWidth = new_width;
            devmode.dmPelsHeight = new_height;
            devmode.dmDisplayFrequency = target_refresh_rate;
            devmode.dmBitsPerPel = 32; // 设置颜色深度，确保模式完整
            unsafe { devmode.Anonymous1.Anonymous2.dmDisplayFixedOutput = DEVMODE_DISPLAY_FIXED_OUTPUT(0); } // 0 = 默认，防止 Windows 进行动态调整

            unsafe {
                devmode.Anonymous1.Anonymous2.dmDisplayOrientation = match orientation {
                    0 => DMDO_DEFAULT,
                    90 => DMDO_90,
                    180 => DMDO_180,
                    270 => DMDO_270,
                    _ => return Err("没有支持的旋转角度".to_string()),
                };
            }

            // 先测试
            let test_result = unsafe {
                ChangeDisplaySettingsExW(device_name, Some(&devmode as *const _), None, CDS_TEST, None)
            };

            if test_result != DISP_CHANGE_SUCCESSFUL {
                return Err("不支持所要求的显示设置".to_string());
            }

            // 应用目标设置
            let result = unsafe {
                ChangeDisplaySettingsExW(
                    device_name,
                    Some(&devmode as *const _),
                    None,
                    CDS_GLOBAL | CDS_UPDATEREGISTRY | CDS_RESET,
                    None,
                )
            };

            match result {
                DISP_CHANGE_SUCCESSFUL => {
                    Ok(())
                },
                DISP_CHANGE_BADMODE => Err("无效显示方式".to_string()),
                DISP_CHANGE_NOTUPDATED => Err("无法应用设置".to_string()),
                DISP_CHANGE_BADFLAGS => Err("无效标志".to_string()),
                _ => Err("未知错误".to_string()),
            }
        } else {
            Err("无法获取显示设置，请检查输入的参数".to_string())
        }
    } else {
        Err("找不到目标显示器，请检查输入的参数".to_string())
    }
}