//! 测量协议与采样器（docs/showcase/01：测量归 benchmark 所有者并脱离 UI）。
//!
//! 渲染图时间戳 span 与百分比统计的唯一家。诊断路径
//! `render/inspector_frame/elapsed_gpu` 是既有 workload/性能报告的数值
//! 身份，名称保留不改（即使它带有 "inspector" 字样）；Inspector 面板与
//! workload runner 都从这里取得同一实现。

use bevy::{
    prelude::*,
    render::{
        RenderApp,
        diagnostic::{RecordDiagnostics, RenderDiagnosticsPlugin},
        renderer::{RenderContext, RenderGraph, RenderGraphSystems},
    },
};
use serde_json::{Value, json};

/// 安装渲染图时间戳测量：在 `render/inspector_frame/elapsed_gpu` 记录
/// 每帧 GPU span（Begin..Render 书挡，子 pass 必须被其包住）。
pub fn install_measurement(app: &mut App) {
    app.add_plugins(RenderDiagnosticsPlugin);
    app.sub_app_mut(RenderApp).add_systems(
        RenderGraph,
        (
            begin
                .after(RenderGraphSystems::Begin)
                .before(RenderGraphSystems::Render),
            end.after(RenderGraphSystems::Render)
                .before(bevy::render::diagnostic::resolve_encoder),
        ),
    );
}

fn begin(mut context: RenderContext) {
    let recorder = context
        .diagnostic_recorder()
        .expect("measurement plugin installed");
    recorder.begin_time_span(context.command_encoder(), "inspector_frame".into());
}

fn end(mut context: RenderContext) {
    let recorder = context
        .diagnostic_recorder()
        .expect("measurement plugin installed");
    recorder.end_time_span(context.command_encoder());
}

/// 百分比统计（count/mean/median/p95/min/max）；空输入为 Null。
pub fn stats(values: &[f64]) -> Value {
    if values.is_empty() {
        return Value::Null;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    json!({"count":n,"mean":values.iter().sum::<f64>()/n as f64,"median":if n.is_multiple_of(2){(sorted[n/2-1]+sorted[n/2])*0.5}else{sorted[n/2]},"p95":sorted[((n as f64*0.95).ceil() as usize).saturating_sub(1)],"min":sorted[0],"max":sorted[n-1]})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentiles_do_not_drop_the_tail() {
        let s = stats(&[1., 2., 3., 100.]);
        assert_eq!(s["p95"], 100.);
        assert_eq!(s["mean"], 26.5);
        assert_eq!(s["median"], 2.5);
        assert!(stats(&[]).is_null());
    }
}
