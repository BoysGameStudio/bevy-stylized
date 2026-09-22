//! 桌面 headless 包装：唯一允许泵送的包装（R1："仅桌面 headless 包装允许
//! pump"）。Android/Web 宿主在各自 runner 中逐帧调用
//! [`CaseDriver::update`] 并自行推进 `app.update()`（phase G）。

use bevy::prelude::App;

use crate::driver::{CaseDriver, DriverStatus};
use crate::error::RuntimeError;

/// 泵送 driver 到终态（Done 或 Failed），或耗尽泵送预算。
///
/// 每轮：先 `driver.update(app)`（非阻塞状态步），再 `app.update()`
/// （宿主渲染泵）。预算是泵送次数不是墙钟——主机总期限由调用方独立
/// 计时（R13）。
pub fn run_to_completion(
    app: &mut App,
    driver: &mut CaseDriver,
    max_pumps: usize,
) -> Result<(), RuntimeError> {
    for _ in 0..max_pumps {
        match driver.update(app.world_mut()) {
            DriverStatus::Running => {
                app.update();
            }
            DriverStatus::Done => return Ok(()),
            DriverStatus::Failed(error) => return Err(error),
        }
    }
    Err(RuntimeError::PumpBudgetExhausted)
}
