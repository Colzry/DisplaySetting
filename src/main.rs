use std::process::Command;
use clap::Parser;
use windows::Win32::Graphics::Gdi::*;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_SETLOGICALDPIOVERRIDE, SPIF_UPDATEINIFILE, SPIF_SENDCHANGE, SendMessageTimeoutW, HWND_BROADCAST, WM_SETTINGCHANGE, SMTO_ABORTIFHUNG};

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

/// 获取所有显示器的信息（包括非活动显示器）
fn get_all_display_devices() -> Vec<DISPLAY_DEVICEW> {
    let mut devices = Vec::new();
    let mut i = 0;

    loop {
        let mut device = DISPLAY_DEVICEW {
            cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
            ..Default::default()
        };

        if unsafe { EnumDisplayDevicesW(None, i, &mut device, 0) }.as_bool() {
            devices.push(device);
            i += 1;
        } else {
            break;
        }
    }

    devices
}

#[allow(dead_code)]
/// 获取活动显示器的信息
fn get_active_display_devices() -> Vec<DISPLAY_DEVICEW> {
    get_all_display_devices()
        .into_iter()
        .filter(|d| (d.StateFlags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP) != 0)
        .collect()
}

/// 设置显示模式 (仅主屏/仅副屏/复制/扩展)
fn set_display_mode(mode: u32) -> Result<(), String> {
    let arg = match mode {
        1 => "/internal",
        2 => "/external",
        3 => "/clone",
        4 => "/extend",
        _ => return Err("无效显示模式".to_string()),
    };

    let status = Command::new("C:\\Windows\\System32\\DisplaySwitch.exe")
        .arg(arg)
        .status()
        .map_err(|e| format!("执行失败: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err("显示模式切换失败".to_string())
    }
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

    let devices = get_all_display_devices();
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


use winreg::enums::*;
use winreg::RegKey;
fn set_display_scaling(dpi: u32) -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let desktop = hkcu.open_subkey_with_flags("Control Panel\\Desktop", KEY_SET_VALUE)
        .map_err(|e| format!("无法打开注册表: {}", e))?;

    // 设置 LogPixels（例如 125% = 120）
    desktop.set_value("LogPixels", &dpi)
        .map_err(|e| format!("无法写入注册表: {}", e))?;

    Ok(())
}

use windows::Win32::UI::Shell::{SHChangeNotify, SHCNE_ASSOCCHANGED, SHCNF_IDLIST};
fn notify_dpi_change() {
    unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM(0),
            LPARAM("WindowsMetrics\0".encode_utf16().collect::<Vec<u16>>().as_ptr() as isize),
            SMTO_ABORTIFHUNG,
            5000,
            *std::ptr::null_mut(),
        );

        // 通知资源管理器设置改变（例如 DPI、文件关联等）
        SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None);
    }
}

fn apply_scaling(scale_percent: u32) -> Result<(), String> {
    let dpi = match scale_percent {
        100 => 96,
        125 => 120,
        150 => 144,
        175 => 168,
        200 => 192,
        _ => return Err("不支持的缩放比例".to_string()),
    };

    set_display_scaling(dpi)?;
    notify_dpi_change();
    println!("缩放已设置为 {}%，请注销后生效", scale_percent);
    Ok(())
}