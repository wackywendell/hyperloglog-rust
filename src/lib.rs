use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub struct HyperLogLog {
    registers: Vec<u32>,
    hasher: DefaultHasher,
}

impl HyperLogLog {
    pub fn new(m: usize) -> HyperLogLog {
        HyperLogLog {
            registers: vec![0; m],
            hasher: DefaultHasher::new(),
        }
    }

    pub fn add<H: Hash>(&mut self, item: H) {
        self.hasher = DefaultHasher::new();
        item.hash(&mut self.hasher);
        let h = self.hasher.finish();
        let m = (h % self.registers.len() as u64) as usize;
        let v = h.leading_zeros();

        if self.registers[m] < v {
            self.registers[m] = v
        };
    }

    fn alpha(m: usize) -> f64 {
        if m == 16 {
            return 0.673;
        } else if m == 32 {
            return 0.697;
        } else if m == 64 {
            return 0.709;
        }
        return 0.7213 / (1.0 + 1.079 / (m as f64));
    }

    fn hll_cardinality(&self) -> f64 {
        let m = self.registers.len();
        let mf64 = m as f64;
        let sum: f64 = self
            .registers
            .iter()
            .map(|mj| 2f64.powf(-(*mj as f64)))
            .sum();
        let z = 1. / sum;
        println!("Sum: {}; z: {}", sum, z);
        return HyperLogLog::alpha(m) * mf64 * mf64 * 2. * z;
    }

    fn linear_count(&self, zero_count: usize) -> f64 {
        let m: f64 = self.registers.len() as f64;
        return m * (m / (zero_count as f64)).ln();
    }

    pub fn count(&self) -> f64 {
        let m = self.registers.len();
        let est = self.hll_cardinality();
        if est > 2.5 * (m as f64) {
            return est;
        }

        // We have an estimate fewer than 5/2 m; may want to try "linear counting"
        let zero_count = self.registers.iter().filter(|&&n| n == 0).count();
        if zero_count == 0 {
            // If there are no zeros, linear_count will be way off
            return est;
        }

        return self.linear_count(zero_count);
    }

    pub fn error_estimate(&self) -> f64 {
        let m = self.registers.len() as f64;
        return 1.04 / m.sqrt();
    }
}

#[cfg(test)]
mod tests {
    use super::HyperLogLog;

    fn assert_close(a: f64, b: f64, err: f64) {
        if (a == 0.) && (b == 0.) {
            return;
        }
        let diff = (a - b).abs();
        let sum = (a.abs() + b.abs()) / 2.;
        assert!(diff / sum < err, "a: {}, b: {}", a, b);
    }

    #[test]
    fn it_works() {
        let mut h = HyperLogLog::new(4);
        assert_eq!(h.count(), 0.);
        assert_close(h.error_estimate(), 0.52, 1e-7);

        let words = vec![
            "Hello!",
            "World!",
            "Hello!",
            "Something!",
            "Else!",
            "Else!",
            "Hello!",
            "1",
            "2",
            "3",
            "4",
            "3",
            "2",
            "1",
        ];

        for w in words {
            h.add(w);
        }

        for v in &h.registers {
            println!("v: {}", v);
        }

        assert_close(h.count(), 4., h.error_estimate() * 3.);
    }

    #[test]
    fn large_test() {
        let mut h = HyperLogLog::new(1 << 8);
        assert_eq!(h.count(), 0.);
        assert!(h.error_estimate() < 0.1);

        let n = 10_000;

        for i in 1..n {
            h.add(i * 3);
        }

        assert_close(h.count(), n as f64, h.error_estimate() * 3.);

        for _ in 1..10 {
            for i in 1..n {
                h.add(i * 3);
            }
        }

        assert_close(h.count(), n as f64, h.error_estimate() * 3.);
    }
}
