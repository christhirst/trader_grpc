use tracing::info;

/// Position sizing calculator for risk management
pub struct PositionSizer {
    /// Maximum percentage of portfolio per trade (e.g., 1.0 = 1%)
    pub max_trade_percent: f64,
    /// Maximum percentage of portfolio per stock (e.g., 10.0 = 10%)
    pub max_position_percent: f64,
}

impl PositionSizer {
    pub fn new(max_trade_percent: f64, max_position_percent: f64) -> Self {
        Self {
            max_trade_percent,
            max_position_percent,
        }
    }

    /// Calculate the number of shares to buy based on portfolio value and risk limits
    ///
    /// # Arguments
    /// * `current_price` - Current price per share
    /// * `portfolio_value` - Total portfolio value (cash + positions)
    /// * `current_position_value` - Current value of position in this stock
    /// * `cash_available` - Available cash
    ///
    /// # Returns
    /// Number of shares to buy (0 if limits would be exceeded)
    pub fn calculate_buy_size(
        &self,
        current_price: f64,
        portfolio_value: f64,
        current_position_value: f64,
        cash_available: f64,
    ) -> i32 {
        if portfolio_value <= 0.0 || current_price <= 0.0 {
            return 0;
        }

        // Calculate max trade value based on portfolio
        let max_trade_value = portfolio_value * (self.max_trade_percent / 100.0);

        // Calculate max position value based on portfolio
        let max_position_value = portfolio_value * (self.max_position_percent / 100.0);

        // Calculate remaining position capacity
        let remaining_position_capacity = max_position_value - current_position_value;

        if remaining_position_capacity <= 0.0 {
            info!("Position limit reached for this stock");
            return 0;
        }

        // Take the minimum of trade limit, position limit, and available cash
        let max_value = max_trade_value
            .min(remaining_position_capacity)
            .min(cash_available);

        // Calculate shares (round down to avoid exceeding limits)
        let shares = (max_value / current_price).floor() as i32;

        shares.max(0)
    }

    /// Calculate the number of shares to sell based on current position
    ///
    /// # Arguments
    /// * `current_position` - Current number of shares held (can be negative for shorts)
    /// * `current_price` - Current price per share
    /// * `portfolio_value` - Total portfolio value
    ///
    /// # Returns
    /// Number of shares to sell
    pub fn calculate_sell_size(
        &self,
        current_position: i32,
        current_price: f64,
        portfolio_value: f64,
    ) -> i32 {
        if current_position > 0 {
            // If we have a long position, sell all of it
            current_position
        } else {
            // If we have no position or a short position, we can't sell more
            // (unless we want to short, which is handled by calculate_short_size)
            0
        }
    }

    /// Calculate the number of shares to short based on portfolio value and risk limits
    ///
    /// # Arguments
    /// * `current_price` - Current price per share
    /// * `portfolio_value` - Total portfolio value
    /// * `current_position` - Current position (negative for shorts)
    ///
    /// # Returns
    /// Number of shares to short (0 if limits would be exceeded)
    pub fn calculate_short_size(
        &self,
        current_price: f64,
        portfolio_value: f64,
        current_position: i32,
    ) -> i32 {
        if portfolio_value <= 0.0 || current_price <= 0.0 {
            return 0;
        }

        // Calculate max trade value based on portfolio
        let max_trade_value = portfolio_value * (self.max_trade_percent / 100.0);

        // Calculate max position value based on portfolio
        let max_position_value = portfolio_value * (self.max_position_percent / 100.0);

        // Calculate current short position value (absolute value)
        let current_short_value = if current_position < 0 {
            (current_position.abs() as f64) * current_price
        } else {
            0.0
        };

        // Calculate remaining short capacity
        let remaining_short_capacity = max_position_value - current_short_value;

        if remaining_short_capacity <= 0.0 {
            info!("Short position limit reached for this stock");
            return 0;
        }

        // Take the minimum of trade limit and position limit
        let max_value = max_trade_value.min(remaining_short_capacity);

        // Calculate shares (round down to avoid exceeding limits)
        let shares = (max_value / current_price).floor() as i32;

        shares.max(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_buy_size_basic() {
        let sizer = PositionSizer::new(1.0, 10.0);

        // Portfolio: $100,000, Price: $100, No position
        // Max trade: $1,000 (1%), Max position: $10,000 (10%)
        // Should buy: 10 shares ($1,000)
        let shares = sizer.calculate_buy_size(100.0, 100_000.0, 0.0, 100_000.0);
        assert_eq!(shares, 10);
    }

    #[test]
    fn test_calculate_buy_size_position_limit() {
        let sizer = PositionSizer::new(1.0, 10.0);

        // Portfolio: $100,000, Price: $100, Current position: $9,500
        // Max trade: $1,000, Remaining capacity: $500
        // Should buy: 5 shares ($500)
        let shares = sizer.calculate_buy_size(100.0, 100_000.0, 9_500.0, 100_000.0);
        assert_eq!(shares, 5);
    }

    #[test]
    fn test_calculate_buy_size_cash_limit() {
        let sizer = PositionSizer::new(1.0, 10.0);

        // Portfolio: $100,000, Price: $100, Cash: $300
        // Max trade: $1,000, but only $300 cash available
        // Should buy: 3 shares ($300)
        let shares = sizer.calculate_buy_size(100.0, 100_000.0, 0.0, 300.0);
        assert_eq!(shares, 3);
    }

    #[test]
    fn test_calculate_buy_size_at_limit() {
        let sizer = PositionSizer::new(1.0, 10.0);

        // Portfolio: $100,000, Price: $100, Current position: $10,000
        // Already at max position
        // Should buy: 0 shares
        let shares = sizer.calculate_buy_size(100.0, 100_000.0, 10_000.0, 100_000.0);
        assert_eq!(shares, 0);
    }

    #[test]
    fn test_calculate_sell_size() {
        let sizer = PositionSizer::new(1.0, 10.0);

        // Sell all shares when we have a position
        let shares = sizer.calculate_sell_size(50, 100.0, 100_000.0);
        assert_eq!(shares, 50);

        // Can't sell if we have no position
        let shares = sizer.calculate_sell_size(0, 100.0, 100_000.0);
        assert_eq!(shares, 0);
    }

    #[test]
    fn test_calculate_short_size_basic() {
        let sizer = PositionSizer::new(1.0, 10.0);

        // Portfolio: $100,000, Price: $100, No position
        // Max trade: $1,000 (1%), Max position: $10,000 (10%)
        // Should short: 10 shares ($1,000)
        let shares = sizer.calculate_short_size(100.0, 100_000.0, 0);
        assert_eq!(shares, 10);
    }

    #[test]
    fn test_calculate_short_size_with_existing_short() {
        let sizer = PositionSizer::new(1.0, 10.0);

        // Portfolio: $100,000, Price: $100, Current short: -95 shares ($9,500)
        // Max trade: $1,000, Remaining capacity: $500
        // Should short: 5 shares ($500)
        let shares = sizer.calculate_short_size(100.0, 100_000.0, -95);
        assert_eq!(shares, 5);
    }

    #[test]
    fn test_calculate_short_size_at_limit() {
        let sizer = PositionSizer::new(1.0, 10.0);

        // Portfolio: $100,000, Price: $100, Current short: -100 shares ($10,000)
        // Already at max short position
        // Should short: 0 shares
        let shares = sizer.calculate_short_size(100.0, 100_000.0, -100);
        assert_eq!(shares, 0);
    }
}
