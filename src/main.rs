pub mod args;
pub mod display;
pub mod logger;
pub mod panic_handler;
mod utils;

use clap::Parser;
use args::Args;
use display::{change_display_settings, get_all_display_devices, get_max_refresh_rate};
use windows::core::PCWSTR;

fn main() {
    panic_handler::setup_panic_handler();

    let args = Args::parse();

    // 检查方向参数有效性
    if let Some(orientation) = args.orientation {
        if ![0, 90, 180, 270].contains(&orientation) {
            ds_info!("无效方向. 允许的值：0、90、180、270。");
            return;
        }
    }

    // 检查显示模式参数有效性
    if let Some(mode) = args.mode {
        if ![1, 2, 3, 4].contains(&mode) {
            ds_error!("无效显示模式. 允许的值: 1(仅主屏), 2(仅副屏), 3(复制), 4(扩展)。");
            return;
        }
    }

    // 处理刷新率
    let effective_refresh_rate = if let Some(rate) = args.refresh_rate {
        if rate == 0 {
            let devices = get_all_display_devices();
            if devices.is_empty() {
                ds_error!("没有检测到任何显示器");
                return;
            }
            let target_index = if args.display as usize <= devices.len() {
                args.display as usize - 1
            } else {
                0
            };
            let device_name = PCWSTR(devices[target_index].DeviceName.as_ptr());
            match get_max_refresh_rate(device_name) {
                Some(max_hz) => {
                    ds_info!("使用最大刷新率: {}Hz", max_hz);
                    Some(max_hz)
                }
                None => {
                    ds_error!("无法获取显示器支持的最大刷新率");
                    return;
                }
            }
        } else {
            ds_info!("使用用户指定刷新率: {}Hz", rate);
            Some(rate)
        }
    } else {
        None
    };

    ds_info!("开始应用显示设置...");

    if let Err(e) = change_display_settings(
        args.display,
        effective_refresh_rate,
        args.orientation,
        args.width,
        args.height,
        args.mode,
    ) {
        ds_error!("错误: {}", e);
    } else {
        ds_info!("显示设置已成功应用。");
    }
}
