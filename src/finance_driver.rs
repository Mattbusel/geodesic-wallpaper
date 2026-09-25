//! Financial Data Geodesic Driver
//!
//! Maps live or historical price data to geodesic parameters.
//!
//! * **Price volatility** (realized standard deviation of log-returns over the
//!   rolling window) drives `speed_multiplier`.
//! * **Trend direction** (normalised slope of the close price over the window)
//!   affects the initial velocity vector components (`theta_velocity`,
//!   `phi_velocity`).
//! * **Volume** (normalized against the window maximum) affects `trail_width`
//!   and feeds into `color_hue`.
//!
//! # Example
//!
//! ```
//! use geodesic_wallpaper::finance_driver::{FinanceDriver, MarketBar};
//!
//! let mut driver = FinanceDriver::new(20);
//! driver.push_bar(MarketBar {
//!     open: 100.0, high: 102.0, low: 99.0, close: 101.5,
//!     volume: 1_000_000.0, timestamp: 0,
//! });
//! let params = driver.compute_params();
//! assert!(params.speed_multiplier >= 0.0);
//! ```

/// A single OHLCV candlestick bar.
#[derive(Debug, Clone)]
pub struct MarketBar {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    /// UNIX timestamp in seconds.
    pub timestamp: u64,
}

/// Geodesic rendering parameters derived from market data.
#[derive(Debug, Clone)]
pub struct GeodesicParams {
    /// Multiplier applied to the RK4 integration speed.
    /// Driven by realized volatility; clamped to `[0.1, 5.0]`.
    pub speed_multiplier: f32,
    /// Initial polar angle θ ∈ \[0, 2π\), derived from latest close price
    /// modulo the surface domain.
    pub initial_theta: f32,
    /// Initial azimuthal angle φ ∈ \[0, 2π\), derived from latest volume.
    pub initial_phi: f32,
    /// Tangent velocity along the θ direction.
    /// Positive for an upward trend, negative for downward.
    pub theta_velocity: f32,
    /// Tangent velocity along the φ direction.
    /// Magnitude proportional to normalized volume.
    pub phi_velocity: f32,
    /// Trail line width in logical pixels. Range `[1.0, 6.0]`.
    pub trail_width: f32,
    /// HSL hue in `[0.0, 1.0]` mapped from price trend.
    /// 0.0 (red) = maximum downtrend; 0.33 (green) = maximum uptrend;
    /// 0.66 (blue) = neutral.
    pub color_hue: f32,
}

impl Default for GeodesicParams {
    fn default() -> Self {
        GeodesicParams {
            speed_multiplier: 1.0,
            initial_theta: 0.0,
            initial_phi: 0.0,
            theta_velocity: 0.1,
            phi_velocity: 0.1,
            trail_width: 2.0,
            color_hue: 0.66,
        }
    }
}

/// Maps OHLCV bars to `GeodesicParams`.
pub struct FinanceDriver {
    bars: Vec<MarketBar>,
    /// Number of bars used for the rolling statistics window.
    window: usize,
    /// Scaling constant that translates normalized volatility to speed units.
    /// Defaults to `3.0`; tune to taste.
    pub c_financial: f64,
}

impl FinanceDriver {
    /// Create a new driver with the given rolling-window size.
    ///
    /// `window` must be at least 2 (a single bar produces no log-return).
    /// Values less than 2 are silently promoted to 2.
    pub fn new(window: usize) -> Self {
        FinanceDriver {
            bars: Vec::new(),
            window: window.max(2),
            c_financial: 3.0,
        }
    }

    /// Push a new bar.  Only the most recent `window` bars are retained.
    pub fn push_bar(&mut self, bar: MarketBar) {
        self.bars.push(bar);
        if self.bars.len() > self.window {
            let excess = self.bars.len() - self.window;
            self.bars.drain(0..excess);
        }
    }

    /// Map the current rolling window to geodesic parameters.
    ///
    /// Returns sensible defaults when fewer than 2 bars are available.
    pub fn compute_params(&self) -> GeodesicParams {
        if self.bars.len() < 2 {
            return GeodesicParams::default();
        }

        let vol = self.volatility();
        let trend = self.trend_direction(); // -1.0 .. +1.0
        let norm_vol = self.normalized_volume();

        // Speed: volatility scaled by c_financial, clamped to [0.1, 5.0].
        let raw_speed = (vol * self.c_financial) as f32;
        let speed_multiplier = raw_speed.clamp(0.1, 5.0).max(0.1);

        // Initial position derived from the latest close and volume.
        let latest = self.bars.last().unwrap();
        let initial_theta = ((latest.close % (2.0 * std::f64::consts::PI)) as f32).abs();
        let initial_phi = ((latest.volume * 1e-6 % (2.0 * std::f64::consts::PI)) as f32).abs();

        // Velocity direction driven by trend.
        let theta_velocity = trend as f32 * 0.3;
        let phi_velocity = norm_vol * 0.2;

        // Trail width: wider on high volume.
        let trail_width = 1.0 + norm_vol * 5.0;

        // Hue: red (0.0) on max downtrend → green (0.33) on max uptrend.
        // Remap trend from [-1, 1] to [0, 0.33].
        let color_hue = ((trend + 1.0) * 0.165) as f32; // 0.0 .. 0.33

        GeodesicParams {
            speed_multiplier,
            initial_theta,
            initial_phi,
            theta_velocity,
            phi_velocity,
            trail_width,
            color_hue,
        }
    }

    /// Parse simple CSV rows: `timestamp,open,high,low,close,volume`.
    ///
    /// Lines starting with `#` or that cannot be parsed are silently skipped.
    /// Returns the number of bars successfully loaded.
    ///
    /// # Example
    ///
    /// ```
    /// use geodesic_wallpaper::finance_driver::FinanceDriver;
    ///
    /// let mut d = FinanceDriver::new(10);
    /// let csv = "1700000000,100.0,102.0,99.0,101.5,500000\n\
    ///            1700000060,101.5,103.0,101.0,102.0,600000\n";
    /// let loaded = d.load_csv(csv);
    /// assert_eq!(loaded, 2);
    /// ```
    pub fn load_csv(&mut self, csv: &str) -> usize {
        let mut count = 0;
        for line in csv.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let parts: Vec<&str> = trimmed.split(',').collect();
            if parts.len() < 6 {
                continue;
            }
            let parsed: Option<MarketBar> = (|| {
                Some(MarketBar {
                    timestamp: parts[0].trim().parse().ok()?,
                    open: parts[1].trim().parse().ok()?,
                    high: parts[2].trim().parse().ok()?,
                    low: parts[3].trim().parse().ok()?,
                    close: parts[4].trim().parse().ok()?,
                    volume: parts[5].trim().parse().ok()?,
                })
            })();
            if let Some(bar) = parsed {
                self.push_bar(bar);
                count += 1;
            }
        }
        count
    }

    // ── Private helpers ────────────────────────────────────────────────────

    /// Realized volatility: population standard deviation of log-returns.
    fn volatility(&self) -> f64 {
        let returns = self.log_returns();
        if returns.is_empty() {
            return 0.0;
        }
        let mean = returns.iter().sum::<f64>() / returns.len() as f64;
        let variance =
            returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / returns.len() as f64;
        variance.sqrt()
    }

    /// Trend direction in `[-1.0, 1.0]`.
    ///
    /// Computed as the sign-and-magnitude of the OLS slope of close prices
    /// normalised by the mean price, clamped to `[-1, 1]`.
    fn trend_direction(&self) -> f64 {
        let closes: Vec<f64> = self.bars.iter().map(|b| b.close).collect();
        let n = closes.len() as f64;
        if n < 2.0 {
            return 0.0;
        }
        let mean_x = (n - 1.0) / 2.0;
        let mean_y = closes.iter().sum::<f64>() / n;
        let num: f64 = closes
            .iter()
            .enumerate()
            .map(|(i, &y)| (i as f64 - mean_x) * (y - mean_y))
            .sum();
        let den: f64 = (0..closes.len() as usize)
            .map(|i| (i as f64 - mean_x).powi(2))
            .sum();
        if den == 0.0 || mean_y == 0.0 {
            return 0.0;
        }
        let slope_normalized = num / den / mean_y;
        slope_normalized.clamp(-1.0, 1.0)
    }

    /// Volume normalized to `[0.0, 1.0]` relative to the window maximum.
    fn normalized_volume(&self) -> f32 {
        let max_vol = self
            .bars
            .iter()
            .map(|b| b.volume)
            .fold(f64::NEG_INFINITY, f64::max);
        if max_vol <= 0.0 {
            return 0.0;
        }
        let latest_vol = self.bars.last().map(|b| b.volume).unwrap_or(0.0);
        (latest_vol / max_vol) as f32
    }

    /// Compute log-returns for the current window.
    fn log_returns(&self) -> Vec<f64> {
        self.bars
            .windows(2)
            .filter_map(|w| {
                let prev = w[0].close;
                let curr = w[1].close;
                if prev > 0.0 && curr > 0.0 {
                    Some((curr / prev).ln())
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bar(close: f64, volume: f64) -> MarketBar {
        MarketBar {
            open: close * 0.99,
            high: close * 1.01,
            low: close * 0.98,
            close,
            volume,
            timestamp: 0,
        }
    }

    #[test]
    fn default_params_on_empty_driver() {
        let d = FinanceDriver::new(10);
        let p = d.compute_params();
        assert!((p.speed_multiplier - 1.0).abs() < 1e-6);
        assert!((p.color_hue - 0.66).abs() < 1e-4);
    }

    #[test]
    fn push_bar_trims_to_window() {
        let mut d = FinanceDriver::new(3);
        for i in 0..10 {
            d.push_bar(make_bar(100.0 + i as f64, 1000.0));
        }
        assert_eq!(d.bars.len(), 3);
    }

    #[test]
    fn load_csv_parses_valid_lines() {
        let mut d = FinanceDriver::new(20);
        let csv = "1700000000,100.0,102.0,99.0,101.5,500000\n\
                   # comment line\n\
                   \n\
                   1700000060,101.5,103.0,101.0,102.0,600000\n";
        let n = d.load_csv(csv);
        assert_eq!(n, 2);
    }

    #[test]
    fn load_csv_skips_bad_rows() {
        let mut d = FinanceDriver::new(20);
        let csv = "not,enough\n\
                   1700000000,100.0,102.0,99.0,101.5,500000\n";
        let n = d.load_csv(csv);
        assert_eq!(n, 1);
    }

    #[test]
    fn speed_multiplier_is_clamped() {
        let mut d = FinanceDriver::new(5);
        // Extreme volatility scenario.
        d.push_bar(make_bar(100.0, 1_000_000.0));
        d.push_bar(make_bar(1.0, 1_000_000.0)); // log-return ≈ -4.6
        d.push_bar(make_bar(1000.0, 1_000_000.0)); // log-return ≈ +6.9
        let p = d.compute_params();
        assert!(p.speed_multiplier <= 5.0);
        assert!(p.speed_multiplier >= 0.1);
    }

    #[test]
    fn trend_direction_uptrend() {
        let mut d = FinanceDriver::new(5);
        for i in 0..5 {
            d.push_bar(make_bar(100.0 + i as f64 * 2.0, 1_000_000.0));
        }
        let p = d.compute_params();
        // Consistent uptrend → positive theta_velocity.
        assert!(p.theta_velocity > 0.0);
        // Hue should be closer to green (0.33) than red (0.0).
        assert!(p.color_hue > 0.1);
    }

    #[test]
    fn normalized_volume_range() {
        let mut d = FinanceDriver::new(5);
        d.push_bar(make_bar(100.0, 500_000.0));
        d.push_bar(make_bar(101.0, 1_000_000.0));
        let p = d.compute_params();
        assert!((0.0..=6.1).contains(&p.trail_width));
    }
}
