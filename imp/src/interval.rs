use core::cmp::{max, min};
use std::collections::BTreeMap;

use util::interner::IdentifierId;

use crate::ai::AbstractDomain;
use crate::ast::ExpressionAST;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interval {
    low: i32,
    high: i32,
}

impl Interval {
    fn top() -> Self {
        Self {
            low: i32::MAX,
            high: i32::MIN,
        }
    }

    fn bottom() -> Self {
        Self {
            low: i32::MIN,
            high: i32::MAX,
        }
    }

    fn join(&self, other: &Interval) -> Self {
        Self {
            low: min(self.low, other.low),
            high: max(self.high, other.high),
        }
    }

    fn meet(&self, other: &Interval) -> Self {
        let met = Self {
            low: max(self.low, other.low),
            high: min(self.high, other.high),
        };
        if met.low > met.high { Self::top() } else { met }
    }

    fn widen(&self, other: &Interval) -> Self {
        Self {
            low: if self.low <= other.low {
                self.low
            } else {
                i32::MIN
            },
            high: if self.high >= other.high {
                self.high
            } else {
                i32::MAX
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct IntervalDomain {
    intervals: BTreeMap<IdentifierId, Interval>,
    finished: Option<Interval>,
}

impl IntervalDomain {
    pub fn new(params: Vec<IdentifierId>) -> Self {
        Self {
            intervals: params
                .into_iter()
                .map(|iden| (iden, Interval::bottom()))
                .collect(),
            finished: None,
        }
    }
}

impl AbstractDomain for IntervalDomain {
    type Value = Interval;

    fn interp_expr(&self, expr: &ExpressionAST<'_>) -> Interval {
        use ExpressionAST::*;
        match expr {
            NumberLiteral(value) => Interval {
                low: *value,
                high: *value,
            },
            Variable(iden) => self.get(*iden),
            Add(lhs, rhs) => {
                let lhs = self.interp_expr(lhs);
                let rhs = self.interp_expr(rhs);
                Interval {
                    low: lhs.low.saturating_add(rhs.low),
                    high: lhs.high.saturating_add(rhs.high),
                }
                //if let (Some(low), Some(high)) = (lhs.low.checked_add(rhs.low), lhs.high.checked_add(rhs.high)) {
                //    Interval { low, high }
                //} else {
                //    Interval::bottom()
                //}
            }
            _ => todo!(),
        }
    }

    fn get(&self, iden: IdentifierId) -> Interval {
        self.intervals[&iden]
    }

    fn assign(&mut self, iden: IdentifierId, val: Interval) {
        self.intervals.insert(iden, val);
    }

    fn branch(&self, _cond: Interval) -> (Self, Self) {
        (self.clone(), self.clone())
    }

    fn finish_with(&mut self, val: Interval) {
        self.finished = Some(val);
    }

    fn join(&self, other: &Self) -> Self {
        assert!(self.finished.is_none());
        assert!(other.finished.is_none());
        let mut intervals = BTreeMap::new();
        for (self_iden, self_interval) in &self.intervals {
            if let Some(other_interval) = other.intervals.get(self_iden) {
                intervals.insert(*self_iden, self_interval.join(other_interval));
            }
        }
        IntervalDomain {
            intervals,
            finished: None,
        }
    }

    fn widen(&self, other: &Self) -> (Self, bool) {
        assert!(self.finished.is_none());
        assert!(other.finished.is_none());
        let mut intervals = BTreeMap::new();
        for (self_iden, self_interval) in &self.intervals {
            if let Some(other_interval) = other.intervals.get(self_iden) {
                intervals.insert(*self_iden, self_interval.widen(other_interval));
            }
        }
        (
            IntervalDomain {
                intervals,
                finished: None,
            },
            false,
        )
    }
}
