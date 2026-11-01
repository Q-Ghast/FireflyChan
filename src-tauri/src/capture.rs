use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

use crate::state::{CropResultDto, MonitorCaptureDto, RectDto};

pub fn monitor_layout() -> Result<Vec<MonitorCaptureDto>, String> {
    let monitors = xcap::Monitor::all().map_err(|error| error.to_string())?;
    let mut layout = Vec::with_capacity(monitors.len());
    for (index, monitor) in monitors.iter().enumerate() {
        layout.push(MonitorCaptureDto {
            index,
            name: monitor.name().unwrap_or_default(),
            x: monitor.x().map_err(|error| error.to_string())?,
            y: monitor.y().map_err(|error| error.to_string())?,
            width: monitor.width().map_err(|error| error.to_string())?,
            height: monitor.height().map_err(|error| error.to_string())?,
            scale: monitor.scale_factor().map_err(|error| error.to_string())? as f64,
            path: String::new(),
        });
    }
    Ok(layout)
}

pub fn capture_all(
    app: &AppHandle,
) -> Result<(String, Vec<MonitorCaptureDto>, PathBuf), String> {
    let session_id = uuid::Uuid::new_v4().to_string();
    let base = app
        .path()
        .app_cache_dir()
        .map_err(|error| error.to_string())?
        .join("captures")
        .join(&session_id);
    std::fs::create_dir_all(&base).map_err(|error| error.to_string())?;

    let monitors = xcap::Monitor::all().map_err(|error| error.to_string())?;
    if monitors.is_empty() {
        return Err("没有找到可截取的显示器".to_string());
    }

    let mut captures = Vec::with_capacity(monitors.len());
    for (index, monitor) in monitors.iter().enumerate() {
        let image = monitor
            .capture_image()
            .map_err(|error| format!("截屏失败（显示器 {index}）：{error}"))?;
        let path = base.join(format!("monitor-{index}.png"));
        image
            .save(&path)
            .map_err(|error| format!("保存截图失败：{error}"))?;

        captures.push(MonitorCaptureDto {
            index,
            name: monitor.name().unwrap_or_default(),
            x: monitor.x().map_err(|error| error.to_string())?,
            y: monitor.y().map_err(|error| error.to_string())?,
            width: image.width(),
            height: image.height(),
            scale: monitor.scale_factor().map_err(|error| error.to_string())? as f64,
            path: path.to_string_lossy().into_owned(),
        });
    }

    Ok((session_id, captures, base))
}

pub fn crop_capture(
    capture: &MonitorCaptureDto,
    rect: &RectDto,
) -> Result<CropResultDto, String> {
    let source = image::open(&capture.path).map_err(|error| error.to_string())?;
    let scale = if capture.scale > 0.0 {
        capture.scale
    } else {
        1.0
    };

    let image_width = source.width();
    let image_height = source.height();

    let x = (rect.x * scale).round().max(0.0) as u32;
    let y = (rect.y * scale).round().max(0.0) as u32;
    if x >= image_width || y >= image_height {
        return Err("选区超出截图范围".to_string());
    }

    let width = ((rect.width * scale).round() as u32)
        .max(1)
        .min(image_width - x);
    let height = ((rect.height * scale).round() as u32)
        .max(1)
        .min(image_height - y);

    let cropped = source.crop_imm(x, y, width, height);

    let parent = Path::new(&capture.path)
        .parent()
        .ok_or_else(|| "截图目录不存在".to_string())?
        .to_path_buf();
    let path = parent.join(format!("shot-{}.png", uuid::Uuid::new_v4()));
    cropped
        .save(&path)
        .map_err(|error| format!("保存裁剪图失败：{error}"))?;

    Ok(CropResultDto {
        path: path.to_string_lossy().into_owned(),
        width: cropped.width(),
        height: cropped.height(),
    })
}
