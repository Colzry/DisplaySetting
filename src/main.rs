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

    /// 显示方向 (0-横向/90-纵向/180-横向翻转/270-纵向翻转)，不传默认不修改
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

    /// 显示模式 (1-仅主屏/2-仅副屏/3-复制/4-扩展)，不传默认不修改
    #[arg(short = 'm', long)]
    mode: Option<u32>,
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

    if let Some(mode) = args.mode {
        if ![1, 2, 3, 4].contains(&mode) {
            eprintln!("无效显示模式. 允许的值: 1(仅主屏), 2(仅副屏), 3(复制), 4(扩展).");
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
        args.mode,
    ) {
        eprintln!("错误: {}", e);
    }
}

/// 获取所有显示器的信息
fn get_display_devices() -> Vec<DISPLAY_DEVICEW> {
    let mut devices = Vec::new();
    let mut i = 0;

    loop {
        let mut device = DISPLAY_DEVICEW {
            cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
            ..Default::default()
        };

        if unsafe { EnumDisplayDevicesW(None, i, &mut device, 0) }.as_bool() {
            if (device.StateFlags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP) != 0 {
                devices.push(device);
            }
            i += 1;
        } else {
            break;
        }
    }

    devices
}

/// 设置显示模式 (仅主屏/仅副屏/复制/扩展)
fn set_display_mode(mode: u32) -> Result<(), String> {
    let devices = get_display_devices();
    if devices.len() < 2{
        return Err("需要至少两个显示器才能设置显示模式".to_string());
    }

    let mut primary_devmode = DEVMODEW {
        dmSize: std::mem::size_of::<DEVMODEW>() as u16,
        ..Default::default()
    };

    let mut secondary_devmode = DEVMODEW {
        dmSize: std::mem::size_of::<DEVMODEW>() as u16,
        ..Default::default()
    };

    let primary_device = PCWSTR(devices[0].DeviceName.as_ptr());
    let secondary_device = PCWSTR(devices[1].DeviceName.as_ptr());

    // 获取当前设置
    if !unsafe { EnumDisplaySettingsW(primary_device, ENUM_CURRENT_SETTINGS, &mut primary_devmode) }.as_bool() {
        return Err("无法获取主显示器设置".to_string());
    }

    if !unsafe { EnumDisplaySettingsW(secondary_device, ENUM_CURRENT_SETTINGS, &mut secondary_devmode) }.as_bool() {
        return Err("无法获取副显示器设置".to_string());
    }

    match mode {
        1 => { // 仅主屏
            unsafe {
                ChangeDisplaySettingsExW(
                    secondary_device,
                    None,
                    None,
                    CDS_UPDATEREGISTRY | CDS_NORESET,
                    None,
                );
                ChangeDisplaySettingsExW(
                    None,
                    None,
                    None,
                    CDS_UPDATEREGISTRY | CDS_RESET,
                    None,
                );
            }
        },
        2 => { // 仅副屏
            unsafe {
                ChangeDisplaySettingsExW(
                    primary_device,
                    None,
                    None,
                    CDS_UPDATEREGISTRY | CDS_NORESET,
                    None,
                );
                ChangeDisplaySettingsExW(
                    None,
                    None,
                    None,
                    CDS_UPDATEREGISTRY | CDS_RESET,
                    None,
                );
            }
        },
        3 => { // 复制模式
            // 使副显示器使用主显示器的设置
            secondary_devmode = primary_devmode.clone();
            secondary_devmode.dmFields = DM_POSITION | DM_PELSWIDTH | DM_PELSHEIGHT | DM_DISPLAYFREQUENCY | DM_DISPLAYFLAGS;

            unsafe {
                ChangeDisplaySettingsExW(
                    secondary_device,
                    Some(&secondary_devmode as *const _),
                    None,
                    CDS_UPDATEREGISTRY | CDS_NORESET,
                    None,
                );
                ChangeDisplaySettingsExW(
                    None,
                    None,
                    None,
                    CDS_UPDATEREGISTRY | CDS_RESET,
                    None,
                );
            }
        },
        4 => { // 扩展模式
            // 设置副显示器在主显示器右侧
            #[allow(unused_unsafe)]
            unsafe {
                secondary_devmode.Anonymous1.Anonymous2.dmPosition.x = primary_devmode.dmPelsWidth as i32;
                secondary_devmode.Anonymous1.Anonymous2.dmPosition.y = 0;
            }
            secondary_devmode.dmFields = DM_POSITION;

            unsafe {
                ChangeDisplaySettingsExW(
                    secondary_device,
                    Some(&secondary_devmode as *const _),
                    None,
                    CDS_UPDATEREGISTRY | CDS_NORESET,
                    None,
                );
                ChangeDisplaySettingsExW(
                    None,
                    None,
                    None,
                    CDS_UPDATEREGISTRY | CDS_RESET,
                    None,
                );
            }
        },
        _ => return Err("无效显示模式".to_string()),
    }

    Ok(())
}

/// 修改显示器的刷新率、方向、缩放比例和分辨率
fn change_display_settings(
    display_index: u32,
    refresh_rate: Option<u32>,
    orientation: Option<u32>,
    scaling: Option<u32>,
    width: Option<u32>,
    height: Option<u32>,
    mode: Option<u32>,
) -> Result<(), String> {
    // 首先处理显示模式（如果指定了）
    if let Some(mode) = mode {
        set_display_mode(mode)?;
    }

    let devices = get_display_devices();
    if devices.is_empty() {
        return Err("没有检测到任何显示器".to_string());
    }

    if display_index as usize > devices.len() {
        return Err(format!("无效显示器索引，当前检测到 {} 个显示器", devices.len()));
    }

    let device = &devices[display_index as usize - 1];
    let mut devmode = DEVMODEW {
        dmSize: std::mem::size_of::<DEVMODEW>() as u16,
        ..Default::default()
    };

    let device_name = PCWSTR(device.DeviceName.as_ptr());

    if unsafe { EnumDisplaySettingsW(device_name, ENUM_CURRENT_SETTINGS, &mut devmode) }.as_bool() {
        let mut changed = false;

        // 处理缩放比例
        if let Some(scale) = scaling {
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

            let log_pixels = match scale {
                100 => 96, 125 => 120, 150 => 144,
                175 => 168, 200 => 192, 225 => 216, 250 => 240,
                _ => return Err("没有支持的缩放值".to_string()),
            };
            devmode.dmFields |= DM_LOGPIXELS;
            devmode.dmLogPixels = log_pixels as u16;
            changed = true;
        }

        // 处理刷新率
        if let Some(target_refresh_rate) = refresh_rate {
            devmode.dmFields |= DM_DISPLAYFREQUENCY;
            devmode.dmDisplayFrequency = target_refresh_rate;
            changed = true;
        }


        // 处理方向
        if let Some(rotate) = orientation {
            // 获取当前方向
            let current_orientation = unsafe { devmode.Anonymous1.Anonymous2.dmDisplayOrientation };

            // 设置新方向
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


            // 确定是否需要交换分辨率
            let need_swap = match (current_orientation, rotate) {
                // 从横向转纵向或反之
                (DMDO_DEFAULT, 90) | (DMDO_DEFAULT, 270) |
                (DMDO_90, 0) | (DMDO_90, 180) |
                (DMDO_180, 90) | (DMDO_180, 270) |
                (DMDO_270, 0) | (DMDO_270, 180) => true,
                _ => false
            };

            // 如果需要交换分辨率且没有显式设置新分辨率
            if need_swap && width.is_none() && height.is_none() {
                let temp = devmode.dmPelsWidth;
                devmode.dmPelsWidth = devmode.dmPelsHeight;
                devmode.dmPelsHeight = temp;
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

        if !changed && mode.is_none() {
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

        // 应用设置
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
}


/// 重启资源管理器
#[allow(dead_code)]
fn restart_explorer_com() {
    Command::new("powershell")
        .args(&["-Command", "(New-Object -ComObject Shell.Application).ToggleDesktop()"])
        .spawn()
        .expect("无法重启 Explorer");
}