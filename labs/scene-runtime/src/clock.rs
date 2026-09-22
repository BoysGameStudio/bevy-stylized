//! 实验时钟（R13）：只由 `fixed_step` 动作推进。
//!
//! 等待（`wait_ready`、观测在途、取消）与宿主泵送次数都不改变本时钟；
//! 这是"不同等待/update 次数和异步到达顺序不改变实验状态"的载体。
//! 整数 tick 与有理步长来自 scenario-model 的 `Tick`/`Step` 合同；
//! 累积实验时间以约分有理数保存，允许混合分母的步长。

use scenario_model::action::Step;

/// 实验时钟。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExperimentClock {
    tick: u64,
    elapsed_num: u128,
    elapsed_den: u128,
}

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a.max(1)
}

impl ExperimentClock {
    /// 当前实验 tick。
    #[must_use]
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// 累积实验时间（约分后的分子/分母；未步进时为 0/1）。
    #[must_use]
    pub fn elapsed(&self) -> (u128, u128) {
        if self.elapsed_den == 0 {
            (0, 1)
        } else {
            (self.elapsed_num, self.elapsed_den)
        }
    }

    /// 推进一个 tick（只有 `fixed_step` 动作的执行路径允许调用），
    /// 按有理步长累积实验时间；返回推进后的 tick。
    pub fn advance(&mut self, step: Step) -> u64 {
        self.tick += 1;
        let numerator = u128::from(step.numerator.0);
        let denominator = u128::from(step.denominator.0);
        if self.elapsed_den == 0 {
            self.elapsed_num = numerator;
            self.elapsed_den = denominator;
        } else {
            self.elapsed_num = self.elapsed_num * denominator + numerator * self.elapsed_den;
            self.elapsed_den *= denominator;
        }
        let divisor = gcd(self.elapsed_num, self.elapsed_den);
        self.elapsed_num /= divisor;
        self.elapsed_den /= divisor;
        self.tick
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_advances_only_via_advance() {
        let mut clock = ExperimentClock::default();
        assert_eq!(clock.tick(), 0);
        assert_eq!(clock.elapsed(), (0, 1));
        assert_eq!(clock.advance(Step::new(1, 60)), 1);
        assert_eq!(clock.advance(Step::new(1, 60)), 2);
        assert_eq!(clock.elapsed(), (1, 30));
    }

    #[test]
    fn mixed_denominators_accumulate_reduced() {
        let mut clock = ExperimentClock::default();
        clock.advance(Step::new(1, 60));
        clock.advance(Step::new(1, 30));
        // 1/60 + 1/30 = 3/60 = 1/20
        assert_eq!(clock.elapsed(), (1, 20));
        clock.advance(Step::new(3, 20));
        assert_eq!(clock.elapsed(), (1, 5));
        assert_eq!(clock.tick(), 3);
    }
}
