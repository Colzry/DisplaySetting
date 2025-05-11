use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::*;
use crate::ds_info;
use crate::utils::{display_mode_to_str, orientation_to_str};

pub fn get_all_display_devices() -> Vec<DISPLAY_DEVICEW> {
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

pub fn get_max_refresh_rate(device_name: PCWSTR) -> Option<u32> {
    let mut max_refresh = 0;
    let mut devmode = DEVMODEW {
        dmSize: std::mem::size_of::<DEVMODEW>() as u16,
        ..Default::default()
    };
    let mut index = 0;
    while unsafe { EnumDisplaySettingsW(device_name, ENUM_DISPLAY_SETTINGS_MODE(index), &mut devmode) }.as_bool() {
        if devmode.dmDisplayFrequency > max_refresh {
            max_refresh = devmode.dmDisplayFrequency;
        }
        index += 1;
    }
    if max_refresh > 0 {
        Some(max_refresh)
    } else {
        None
    }
}

fn set_display_mode(mode: u32) -> Result<(), String> {
    use std::process::Command;

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

fn is_dev_mode_equal(current: &DEVMODEW, new: &DEVMODEW) -> bool {
    current.dmPelsWidth == new.dmPelsWidth &&
        current.dmPelsHeight == new.dmPelsHeight &&
        current.dmBitsPerPel == new.dmBitsPerPel &&
        current.dmDisplayFrequency == new.dmDisplayFrequency &&
        unsafe { current.Anonymous1.Anonymous2.dmDisplayOrientation } ==
            unsafe { new.Anonymous1.Anonymous2.dmDisplayOrientation }
}

#[allow(unused_unsafe)]
pub fn change_display_settings(
    display_index: u32,
    refresh_rate: Option<u32>,
    orientation: Option<u32>,
    width: Option<u32>,
    height: Option<u32>,
    mode: Option<u32>,
) -> Result<(), String> {
    if let Some(mode) = mode {
        ds_info!("正在切换显示模式: {:?}", display_mode_to_str(mode));
        set_display_mode(mode)?;
        ds_info!("显示模式切换成功");
    }

    let devices = get_all_display_devices();
    if devices.is_empty() {
        return Err("没有检测到任何显示器".to_string());
    }

    if display_index as usize > devices.len() {
        return Err(format!(
            "无效显示器索引，当前检测到 {} 个显示器",
            devices.len()
        ));
    }

    let device = &devices[display_index as usize - 1];
    #[allow(unused_assignments)]
    let mut desired_devmode  = DEVMODEW {
        dmSize: std::mem::size_of::<DEVMODEW>() as u16,
        ..Default::default()
    };

    let device_name = PCWSTR(device.DeviceName.as_ptr());
    // 获取当前设置
    let mut current_devmode = DEVMODEW {
        dmSize: std::mem::size_of::<DEVMODEW>() as u16,
        ..Default::default()
    };
    if !unsafe { EnumDisplaySettingsW(device_name, ENUM_CURRENT_SETTINGS, &mut current_devmode) }.as_bool() {
        return Err("无法获取当前显示设置".to_string());
    }
    // 复制当前设置作为基础
    desired_devmode = current_devmode;

    // 刷新率
    if let Some(target_refresh_rate) = refresh_rate {
        desired_devmode.dmFields |= DM_DISPLAYFREQUENCY;
        desired_devmode.dmDisplayFrequency = target_refresh_rate;
    }

    // 方向
    if let Some(rotate) = orientation {
        unsafe {
            desired_devmode.Anonymous1.Anonymous2.dmDisplayOrientation = match rotate {
                0 => DMDO_DEFAULT,
                90 => DMDO_90,
                180 => DMDO_180,
                270 => DMDO_270,
                _ => return Err("没有支持的旋转角度".to_string()),
            };
        }
        desired_devmode.dmFields |= DM_DISPLAYORIENTATION;
        let need_swap = match (
            unsafe { current_devmode.Anonymous1.Anonymous2.dmDisplayOrientation },
            rotate,
        ) {
            (DMDO_DEFAULT, 90) | (DMDO_DEFAULT, 270)
            | (DMDO_90, 0) | (DMDO_90, 180)
            | (DMDO_180, 90) | (DMDO_180, 270)
            | (DMDO_270, 0) | (DMDO_270, 180) => true,
            _ => false,
        };
        if need_swap && width.is_none() && height.is_none() {
            let temp = desired_devmode.dmPelsWidth;
            desired_devmode.dmPelsWidth = desired_devmode.dmPelsHeight;
            desired_devmode.dmPelsHeight = temp;
        }
        ds_info!("正在设置屏幕方向为: {}", orientation_to_str(rotate));
    }

    // 分辨率
    if let (Some(w), Some(h)) = (width, height) {
        desired_devmode.dmFields |= DM_PELSWIDTH | DM_PELSHEIGHT;
        desired_devmode.dmPelsWidth = w;
        desired_devmode.dmPelsHeight = h;
        ds_info!("正在设置分辨率为: {}x{}", w, h);
    }

    // 增加额外字段
    desired_devmode.dmFields |= DM_BITSPERPEL;
    desired_devmode.dmBitsPerPel = 32;
    unsafe {
        desired_devmode.Anonymous1.Anonymous2.dmDisplayFixedOutput =
            DEVMODE_DISPLAY_FIXED_OUTPUT(0);
    }
    // 检查是否和当前设置一样
    if is_dev_mode_equal(&current_devmode, &desired_devmode) {
        ds_info!("显示设置未发生改变，跳过设置");
        return Ok(());
    }
    // 测试是否支持
    let test_result = unsafe {
        ChangeDisplaySettingsExW(device_name, Some(&desired_devmode as *const _), None, CDS_TEST, None)
    };
    if test_result != DISP_CHANGE_SUCCESSFUL {
        return Err("不支持所要求的显示设置".to_string());
    }
    // 应用设置
    let result = unsafe {
        ChangeDisplaySettingsExW(
            device_name,
            Some(&desired_devmode as *const _),
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
}
