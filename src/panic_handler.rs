use crate::ds_error;

pub fn setup_panic_handler() {
    std::panic::set_hook(Box::new(|info| {
        let location = info.location().unwrap();
        let msg = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => s.to_string(),
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => s.clone(),
                None => "未知 panic 内容".to_string(),
            },
        };

        ds_error!("PANIC at {}:{}\n{}", location.file(), location.line(), msg);
    }));
}
