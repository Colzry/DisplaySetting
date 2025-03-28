use std::process::Command;
use clap::Parser;
use windows::Win32::Graphics::Gdi::*;
use windows::core::PCWSTR;
use windows::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_SETLOGICALDPIOVERRIDE, SPIF_UPDATEINIFILE, SPIF_SENDCHANGE};


#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// 刷新率 (Hz)，不传默认不修改
    #[arg(short = 'r', long)]
    refresh_rate: Option<u32>,

    /// 显示方向 (0-横向/90-纵向/180-横向翻转/270-纵向翻转)，默认 0
    #[arg(short = 'o', long)]
    orientation: Option<u32>,

    /// 目标显示器 (1-显示器1/2-显示器2)，默认 1
    #[arg(short = 'd', long, default_value_t = 1)]
    display: u32,

    /// 缩放比例 (100/125/150/175/200/225/250)，不传默认不修改
    #[arg(short = 's', long)]
    scaling: Option<u32>,

    /// 目标分辨率宽度，需和高度一起设置，不传默认不修改
    #[arg(short = 'w', long)]
    width: Option<u32>,

    /// 目标分辨率高度，需和宽度一起设置，不传默认不修改
    #[arg(short = 'h', long)]
    height: Option<u32>,
}

fn main() {
    let args = Args::parse();

    if let Some(orientation) = args.orientation {
        if ![0, 90, 180, 270].contains(&orientation) {
            eprintln!("无效方向. 允许的值：0、90、180、270.");
            return;
        }
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
#[allow(dead_code)]
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

/// 重启资源管理器
#[allow(dead_code)]
fn restart_explorer_com() {
    Command::new("powershell")
        .args(&["-Command", "(New-Object -ComObject Shell.Application).ToggleDesktop()"])
        .spawn()
        .expect("无法重启 Explorer");
}

/// 修改显示器的刷新率、方向、缩放比例和分辨率
fn change_display_settings(
    display_index: u32,
    refresh_rate: Option<u32>,
    orientation: Option<u32>,
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
            let mut changed = false;

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

            // 处理刷新率
            if let Some(target_refresh_rate) = refresh_rate {
                devmode.dmFields |= DM_DISPLAYFREQUENCY;
                devmode.dmDisplayFrequency = target_refresh_rate;
                changed = true;
            }

            // 处理方向
            if let Some(rotate) = orientation {
                devmode.dmFields |= DM_DISPLAYORIENTATION;
                #[allow(unused_unsafe)]
                unsafe {
                    devmode.Anonymous1.Anonymous2.dmDisplayOrientation = match rotate {
                        0 => DMDO_DEFAULT,
                        90 => DMDO_90,
                        180 => DMDO_180,
                        270 => DMDO_270,
                        _ => return Err("没有支持的旋转角度".to_string()),
                    };
                };

                // 如果用户未提供 width 和 height，自动交换当前分辨率
                // 只有90°和270°才交换宽高
                if width.is_none() && height.is_none() {
                    if rotate == 90 || rotate == 270 {
                        let temp = devmode.dmPelsWidth;
                        devmode.dmPelsWidth = devmode.dmPelsHeight;
                        devmode.dmPelsHeight = temp;
                    }
                }
                changed = true;
            }

            // 处理分辨率
            if width.is_some() && height.is_some() {
                devmode.dmFields |= DM_PELSWIDTH | DM_PELSHEIGHT;
                devmode.dmPelsWidth = width.unwrap();
                devmode.dmPelsHeight = height.unwrap();
                changed = true;
            }

            // 如果没有任何改动，则直接返回成功
            if !changed {
                return Ok(());
            }

            devmode.dmFields |= DM_BITSPERPEL;
            devmode.dmBitsPerPel = 32;
            #[allow(unused_unsafe)]
            unsafe { devmode.Anonymous1.Anonymous2.dmDisplayFixedOutput = DEVMODE_DISPLAY_FIXED_OUTPUT(0); }

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